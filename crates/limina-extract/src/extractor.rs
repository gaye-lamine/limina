//! The extraction pipeline: cache, rate limit, one retry, strict parsing,
//! then the deterministic checks against the actual text.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::catalog::Catalog;
use crate::check::{check, CheckedClaim};
use crate::error::ExtractError;
use crate::parse::{read_reply, ModelOutput};
use crate::prompt;
use crate::provider::{ChatReply, ExtractionProvider, Route};

/// Requests allowed per window.
pub const RATE_LIMIT: usize = 30;
/// Length of the rate-limit window.
pub const RATE_WINDOW: Duration = Duration::from_secs(60);
/// Pause before the one retry on HTTP 429 or 5xx.
pub const DEFAULT_RETRY_BACKOFF: Duration = Duration::from_millis(800);
/// Cached answers kept; the cache is emptied when full.
const CACHE_CAPACITY: usize = 256;

/// One extraction, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Extraction {
    /// The claims after the deterministic checks.
    pub claims: Vec<CheckedClaim>,
    /// Values the model said the text does not give.
    pub missing: Vec<String>,
    /// Model id that answered (the first of the chain unless it fell back).
    pub model: String,
    /// Models passed over before it, with the reason of each; empty when
    /// the first model answered.
    pub skipped: Vec<String>,
    /// Whether the model answer came from the cache.
    pub cached: bool,
    /// Time spent, in milliseconds.
    pub elapsed_ms: u64,
}

/// Reads claims out of texts through a provider.
pub struct Extractor {
    provider: Arc<dyn ExtractionProvider>,
    text_provider: Option<Arc<dyn ExtractionProvider>>,
    json_provider: Option<Arc<dyn ExtractionProvider>>,
    catalog: Catalog,
    retry_backoff: Option<Duration>,
    cache: Mutex<HashMap<[u8; 32], (ModelOutput, Route)>>,
    calls: Mutex<VecDeque<Instant>>,
}

impl std::fmt::Debug for Extractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extractor")
            .field("model", &self.provider.model())
            .finish_non_exhaustive()
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Reason given when a model answer quotes none of its values from the text.
pub const UNTRACEABLE_ANSWER: &str = "no value of the answer is written in the text";

/// Whether the answer reads at least one claim and none of its claim values
/// is traceable to the text (evidence not found, or the number not in it).
/// An answer with no claim is not untraceable: the text may state none.
pub fn untraceable(output: &ModelOutput, text: &str, catalog: &Catalog) -> bool {
    let claims = check(output, text, catalog);
    !claims.is_empty() && claims.iter().all(|c| c.evidence.is_none())
}

/// The text with its outer whitespace trimmed and every run of whitespace
/// made one space: the cache key is the SHA-256 of this.
pub fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cache_key(text: &str) -> [u8; 32] {
    Sha256::digest(normalize_text(text).as_bytes()).into()
}

impl Extractor {
    /// An extractor for these limits, with one retry after [`DEFAULT_RETRY_BACKOFF`].
    pub fn new(provider: Arc<dyn ExtractionProvider>, catalog: Catalog) -> Extractor {
        Extractor {
            provider,
            text_provider: None,
            json_provider: None,
            catalog,
            retry_backoff: Some(DEFAULT_RETRY_BACKOFF),
            cache: Mutex::new(HashMap::new()),
            calls: Mutex::new(VecDeque::new()),
        }
    }

    /// Sets the pause before the retry; None disables the retry.
    pub fn with_retry_backoff(mut self, backoff: Option<Duration>) -> Extractor {
        self.retry_backoff = backoff;
        self
    }

    /// Answers law naming with this provider (a chain that accepts a plain
    /// sentence) instead of the JSON provider.
    pub fn with_text_provider(mut self, provider: Arc<dyn ExtractionProvider>) -> Extractor {
        self.text_provider = Some(provider);
        self
    }
    /// Model id.
    pub fn model(&self) -> &str {
        self.provider.model()
    }
    /// Answers the other JSON questions (parameters, authoring model A) with
    /// this provider, when the main provider checks the extraction schema.
    pub fn with_json_provider(mut self, provider: Arc<dyn ExtractionProvider>) -> Extractor {
        self.json_provider = Some(provider);
        self
    }

    /// The provider of JSON answers other than the extraction.
    pub(crate) fn json_provider(&self) -> &dyn ExtractionProvider {
        self.json_provider
            .as_deref()
            .unwrap_or_else(|| self.provider.as_ref())
    }

    /// The provider of free-text answers (law naming).
    pub(crate) fn text_provider(&self) -> &dyn ExtractionProvider {
        self.text_provider
            .as_deref()
            .unwrap_or_else(|| self.provider.as_ref())
    }

    pub(crate) fn retry_backoff(&self) -> Option<Duration> {
        self.retry_backoff
    }

    /// The limits metadata the prompt is built from.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Takes one slot of the rate limit, or fails with `RateLimited`.
    pub(crate) fn acquire(&self) -> Result<(), ExtractError> {
        let now = Instant::now();
        let mut calls = lock(&self.calls);
        while calls
            .front()
            .is_some_and(|t| now.duration_since(*t) >= RATE_WINDOW)
        {
            calls.pop_front();
        }
        if calls.len() >= RATE_LIMIT {
            return Err(ExtractError::RateLimited);
        }
        calls.push_back(now);
        Ok(())
    }

    async fn ask(&self, text: &str) -> Result<(ModelOutput, Route), ExtractError> {
        let request = prompt::request(&self.catalog, text);
        let quotes = |reply: &ChatReply| -> Option<String> {
            let output = read_reply(reply).ok()?;
            untraceable(&output, text, &self.catalog).then(|| UNTRACEABLE_ANSWER.to_string())
        };
        let first = self
            .provider
            .complete_routed_checked(request.clone(), &quotes)
            .await;
        let routed = match (first, self.retry_backoff) {
            (Err(e), Some(backoff)) if e.is_retryable() => {
                tokio::time::sleep(backoff).await;
                self.provider
                    .complete_routed_checked(request, &quotes)
                    .await?
            }
            (other, _) => other?,
        };
        Ok((read_reply(&routed.reply)?, routed.route))
    }

    /// Extracts and checks the claims of `text`. A cached model answer is
    /// reused for the same normalized text; the checks always run against
    /// the text given here.
    pub async fn extract(&self, text: &str) -> Result<Extraction, ExtractError> {
        let started = Instant::now();
        let key = cache_key(text);
        let hit = lock(&self.cache).get(&key).cloned();
        let ((output, route), cached) = match hit {
            Some(entry) => (entry, true),
            None => {
                self.acquire()?;
                let output = self.ask(text).await?;
                // An answer none of whose values is written in the text is
                // not kept: the next request asks the models again.
                if !untraceable(&output.0, text, &self.catalog) {
                    let mut cache = lock(&self.cache);
                    if cache.len() >= CACHE_CAPACITY {
                        cache.clear();
                    }
                    cache.insert(key, output.clone());
                }
                (output, false)
            }
        };
        Ok(Extraction {
            claims: check(&output, text, &self.catalog),
            missing: output.missing.clone(),
            model: route.model,
            skipped: route.skipped,
            cached,
            elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatReply, ProviderError};
    use crate::testing::{catalog, MockProvider};

    const OUT: &str = r#"{"claims":[],"missing":["Th"]}"#;

    fn ok() -> Result<ChatReply, ProviderError> {
        Ok(ChatReply {
            content: Some(OUT.to_string()),
            finish_reason: Some("stop".to_string()),
        })
    }

    fn extractor(mock: &Arc<MockProvider>) -> Extractor {
        Extractor::new(mock.clone(), catalog()).with_retry_backoff(Some(Duration::ZERO))
    }

    const BETZ_TEXT: &str = "With an unshrouded rotor, a power coefficient of 0.45.";

    fn betz_answer(evidence: &str) -> String {
        format!(
            r#"{{"claims":[{{"limit_id":"betz","quantity":"power coefficient","value":0.45,"unit":"dimensionless","evidence":"{evidence}","constraints":{{}},"parameters":{{}}}}],"missing":[]}}"#
        )
    }

    fn chain(a: &Arc<crate::testing::NamedMock>, b: &Arc<crate::testing::NamedMock>) -> Extractor {
        let links: Vec<Arc<dyn ExtractionProvider>> = vec![a.clone(), b.clone()];
        let chain = crate::fallback::FallbackProvider::new(
            links,
            crate::fallback::ReplyKind::Checked(crate::fallback::extraction_problem),
            Duration::from_secs(5),
        );
        Extractor::new(Arc::new(chain), catalog()).with_retry_backoff(Some(Duration::ZERO))
    }

    #[tokio::test]
    async fn an_answer_that_quotes_nothing_of_the_text_moves_to_the_next_model() {
        let a = Arc::new(crate::testing::NamedMock::new("model-a"));
        let b = Arc::new(crate::testing::NamedMock::new("model-b"));
        a.inner
            .push_content(&betz_answer("a power coefficient of 0,4 5"));
        b.inner
            .push_content(&betz_answer("a power coefficient of 0.45"));
        let e = chain(&a, &b).extract(BETZ_TEXT).await.expect("extracted");
        assert_eq!(e.model, "model-b");
        assert!(e.claims[0].evidence.is_some());
        assert_eq!(e.skipped.len(), 1);
        assert!(e.skipped[0].contains(UNTRACEABLE_ANSWER), "{:?}", e.skipped);
    }

    #[tokio::test]
    async fn when_no_model_quotes_the_text_the_first_reading_is_reported_and_not_cached() {
        let a = Arc::new(crate::testing::NamedMock::new("model-a"));
        let b = Arc::new(crate::testing::NamedMock::new("model-b"));
        a.inner.push_content(&betz_answer("not in the text 0.45"));
        b.inner
            .push_content(&betz_answer("also not in the text 0.45"));
        let x = chain(&a, &b);
        let e = x.extract(BETZ_TEXT).await.expect("reported");
        assert_eq!(e.model, "model-a", "the first well-formed reading is kept");
        assert!(e.claims[0].evidence.is_none(), "and shown as not traceable");
        assert!(!e.cached);
        // Not cached: the next request asks the models again.
        a.inner
            .push_content(&betz_answer("a power coefficient of 0.45"));
        let again = x.extract(BETZ_TEXT).await.expect("asked again");
        assert!(!again.cached);
        assert!(again.claims[0].evidence.is_some());
        // A good answer is cached.
        assert!(x.extract(BETZ_TEXT).await.expect("cached").cached);
        assert_eq!(a.inner.calls(), 2);
        assert_eq!(b.inner.calls(), 1);
    }

    #[test]
    fn an_answer_without_claims_is_not_untraceable() {
        let none = crate::parse::parse_model_output(r#"{"claims":[],"missing":[]}"#).expect("json");
        assert!(!untraceable(&none, BETZ_TEXT, &catalog()));
    }

    #[tokio::test]
    async fn http_429_then_success_is_retried_once() {
        let mock = Arc::new(MockProvider::new());
        mock.push(Err(ProviderError::Status(429)));
        mock.push(ok());
        let e = extractor(&mock).extract("t").await.expect("retried");
        assert_eq!(e.missing, vec!["Th"]);
        assert_eq!(mock.calls(), 2);
    }

    #[tokio::test]
    async fn a_5xx_twice_fails_after_one_retry() {
        let mock = Arc::new(MockProvider::new());
        mock.push(Err(ProviderError::Status(502)));
        mock.push(Err(ProviderError::Status(503)));
        let err = extractor(&mock)
            .extract("t")
            .await
            .expect_err("two failures");
        assert_eq!(err, ExtractError::Provider(ProviderError::Status(503)));
        assert_eq!(mock.calls(), 2);
    }

    #[tokio::test]
    async fn other_errors_are_not_retried() {
        let mock = Arc::new(MockProvider::new());
        mock.push(Err(ProviderError::Status(401)));
        assert!(extractor(&mock).extract("t").await.is_err());
        assert_eq!(mock.calls(), 1);
    }

    #[tokio::test]
    async fn the_same_normalized_text_is_a_cache_hit() {
        let mock = Arc::new(MockProvider::new());
        mock.push(ok());
        let x = extractor(&mock);
        let first = x.extract("  a heat   engine ").await.expect("first");
        let second = x.extract("a heat engine").await.expect("second");
        assert!(!first.cached);
        assert!(second.cached);
        assert_eq!(mock.calls(), 1);
        assert_eq!(second.model, "mock-model");
    }

    #[tokio::test]
    async fn the_31st_request_in_a_minute_is_rate_limited() {
        let mock = Arc::new(MockProvider::new());
        for _ in 0..RATE_LIMIT {
            mock.push(ok());
        }
        let x = extractor(&mock);
        for i in 0..RATE_LIMIT {
            x.extract(&format!("text {i}"))
                .await
                .expect("within the limit");
        }
        assert_eq!(x.extract("one more").await, Err(ExtractError::RateLimited));
        assert_eq!(mock.calls(), RATE_LIMIT);
        // A cached text still answers.
        assert!(x.extract("text 0").await.expect("cached").cached);
    }

    #[test]
    fn normalization_trims_and_collapses_whitespace() {
        assert_eq!(normalize_text("  a\n\t b  c "), "a b c");
        assert_eq!(cache_key("a  b"), cache_key(" a b"));
        assert_ne!(cache_key("a b"), cache_key("a c"));
    }
}
