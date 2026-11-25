//! A chain of models behind the provider traits.
//!
//! A model that fails (timeout, transport, HTTP 404, 429 or 5xx) or gives an
//! unusable answer (empty, cut at the token limit, no balanced JSON object,
//! a JSON object that does not parse) is passed over and the next model of
//! the chain is tried. A model that failed on the provider side is skipped
//! for [`COOL_DOWN_SECS`] seconds (circuit breaker); an unusable answer does
//! not open the breaker, because it can come from one odd request. When
//! every model is cooling down they are all tried anyway, so a stale breaker
//! never blocks the pipeline. The chain only moves text around: it never
//! reads a verdict and holds no key.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::COOL_DOWN_SECS;
use crate::parse::{first_json_object, strip_fences};
use crate::provider::{
    BoxFuture, ChatReply, ChatRequest, ExtractionProvider, ImageInput, ProviderError, Route,
    Routed, VisionProvider,
};

/// What an answer must look like to be usable.
#[derive(Debug, Clone, Copy)]
pub enum ReplyKind {
    /// Non-empty, not cut, and holding one JSON object that parses.
    Json,
    /// Non-empty and not cut (a sentence or a transcription).
    Text,
    /// A JSON answer that must also pass this check (for example the
    /// extraction schema); the check returns the reason it fails.
    Checked(fn(&ChatReply) -> Option<&'static str>),
}

/// The reason an extraction answer does not fit the extraction schema, or
/// None. For [`ReplyKind::Checked`] on the extraction chain.
pub fn extraction_problem(reply: &ChatReply) -> Option<&'static str> {
    match crate::parse::read_reply(reply) {
        Ok(_) => None,
        Err(crate::error::ExtractError::ExtractionFailed(reason)) => Some(reason),
        Err(_) => Some("the answer does not match the schema"),
    }
}

/// The reason an answer is unusable, or None.
pub fn unusable(kind: ReplyKind, reply: &ChatReply) -> Option<&'static str> {
    if reply.finish_reason.as_deref() == Some("length") {
        return Some("answer cut at the token limit");
    }
    let content = reply.content.as_deref().unwrap_or("").trim();
    if content.is_empty() {
        return Some("empty answer");
    }
    if matches!(kind, ReplyKind::Text) {
        return None;
    }
    let unfenced = strip_fences(content);
    let Some(object) = first_json_object(&unfenced) else {
        return Some("no JSON object in the answer");
    };
    match serde_json::from_str::<serde_json::Value>(object) {
        Ok(v) if v.is_object() => match kind {
            ReplyKind::Checked(check) => check(reply),
            _ => None,
        },
        _ => Some("the JSON object does not parse"),
    }
}

/// Whether an error moves the chain to the next model.
fn falls_through(e: &ProviderError) -> bool {
    match e {
        ProviderError::Timeout | ProviderError::Transport | ProviderError::Unusable(_) => true,
        ProviderError::Status(s) => *s == 404 || *s == 429 || (500..=599).contains(s),
    }
}

fn why(e: &ProviderError) -> String {
    match e {
        ProviderError::Timeout => "timeout".to_string(),
        ProviderError::Transport => "unreachable".to_string(),
        ProviderError::Status(s) => format!("HTTP {s}"),
        ProviderError::Unusable(r) => (*r).to_string(),
    }
}

/// The circuit breaker: when each model last failed on the provider side.
#[derive(Debug)]
struct Breaker {
    cool_down: Duration,
    failed_at: Mutex<HashMap<usize, Instant>>,
}

impl Breaker {
    fn new(cool_down: Duration) -> Breaker {
        Breaker {
            cool_down,
            failed_at: Mutex::new(HashMap::new()),
        }
    }
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<usize, Instant>> {
        self.failed_at.lock().unwrap_or_else(|p| p.into_inner())
    }
    fn is_open(&self, index: usize) -> bool {
        self.map()
            .get(&index)
            .is_some_and(|t| t.elapsed() < self.cool_down)
    }
    fn trip(&self, index: usize) {
        self.map().insert(index, Instant::now());
    }
}

/// A chain of chat models: the first usable answer wins.
pub struct FallbackProvider {
    links: Vec<Arc<dyn ExtractionProvider>>,
    kind: ReplyKind,
    timeout: Duration,
    breaker: Breaker,
}

impl std::fmt::Debug for FallbackProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let models: Vec<&str> = self.links.iter().map(|l| l.model()).collect();
        f.debug_struct("FallbackProvider")
            .field("models", &models)
            .field("kind", &self.kind)
            .finish()
    }
}

impl FallbackProvider {
    /// A chain, tried in order, each model given `timeout`.
    pub fn new(
        links: Vec<Arc<dyn ExtractionProvider>>,
        kind: ReplyKind,
        timeout: Duration,
    ) -> FallbackProvider {
        FallbackProvider {
            links,
            kind,
            timeout,
            breaker: Breaker::new(Duration::from_secs(COOL_DOWN_SECS)),
        }
    }
    /// The same chain with another cool-down after a failure.
    pub fn with_cool_down(mut self, cool_down: Duration) -> FallbackProvider {
        self.breaker = Breaker::new(cool_down);
        self
    }
    /// Model ids of the chain, in order.
    pub fn models(&self) -> Vec<String> {
        self.links.iter().map(|l| l.model().to_string()).collect()
    }

    async fn run(&self, request: ChatRequest) -> Result<Routed, ProviderError> {
        let all_open = (0..self.links.len()).all(|i| self.breaker.is_open(i));
        let mut skipped: Vec<String> = Vec::new();
        let mut last: Option<ProviderError> = None;
        for (i, link) in self.links.iter().enumerate() {
            let model = link.model().to_string();
            if !all_open && self.breaker.is_open(i) {
                skipped.push(format!("{model} (cooling down after a failure)"));
                continue;
            }
            let answer = tokio::time::timeout(self.timeout, link.complete(request.clone()))
                .await
                .unwrap_or(Err(ProviderError::Timeout));
            match answer {
                Ok(reply) => match unusable(self.kind, &reply) {
                    None => {
                        return Ok(Routed {
                            reply,
                            route: Route { model, skipped },
                        })
                    }
                    Some(reason) => {
                        skipped.push(format!("{model} ({reason})"));
                        last = Some(ProviderError::Unusable(reason));
                    }
                },
                Err(e) if falls_through(&e) => {
                    self.breaker.trip(i);
                    skipped.push(format!("{model} ({})", why(&e)));
                    last = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or(ProviderError::Unusable("no model configured")))
    }
}

impl ExtractionProvider for FallbackProvider {
    fn model(&self) -> &str {
        self.links.first().map_or("", |l| l.model())
    }
    fn complete(&self, request: ChatRequest) -> BoxFuture<'_, Result<ChatReply, ProviderError>> {
        Box::pin(async move { self.run(request).await.map(|r| r.reply) })
    }
    fn complete_routed(
        &self,
        request: ChatRequest,
    ) -> BoxFuture<'_, Result<Routed, ProviderError>> {
        Box::pin(self.run(request))
    }
}

/// A chain of vision models for image transcription.
pub struct FallbackVision {
    links: Vec<Arc<dyn VisionProvider>>,
    timeout: Duration,
    breaker: Breaker,
}

impl FallbackVision {
    /// A chain, tried in order, each model given `timeout`.
    pub fn new(links: Vec<Arc<dyn VisionProvider>>, timeout: Duration) -> FallbackVision {
        FallbackVision {
            links,
            timeout,
            breaker: Breaker::new(Duration::from_secs(COOL_DOWN_SECS)),
        }
    }
    /// Model ids of the chain, in order.
    pub fn models(&self) -> Vec<String> {
        self.links.iter().map(|l| l.model().to_string()).collect()
    }

    async fn run(&self, image: ImageInput) -> Result<Routed, ProviderError> {
        let all_open = (0..self.links.len()).all(|i| self.breaker.is_open(i));
        let mut skipped: Vec<String> = Vec::new();
        let mut last: Option<ProviderError> = None;
        for (i, link) in self.links.iter().enumerate() {
            let model = link.model().to_string();
            if !all_open && self.breaker.is_open(i) {
                skipped.push(format!("{model} (cooling down after a failure)"));
                continue;
            }
            let answer = tokio::time::timeout(self.timeout, link.transcribe(image.clone()))
                .await
                .unwrap_or(Err(ProviderError::Timeout));
            match answer {
                Ok(reply) => match unusable(ReplyKind::Text, &reply) {
                    None => {
                        return Ok(Routed {
                            reply,
                            route: Route { model, skipped },
                        })
                    }
                    Some(reason) => {
                        skipped.push(format!("{model} ({reason})"));
                        last = Some(ProviderError::Unusable(reason));
                    }
                },
                Err(e) if falls_through(&e) => {
                    self.breaker.trip(i);
                    skipped.push(format!("{model} ({})", why(&e)));
                    last = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or(ProviderError::Unusable("no model configured")))
    }
}

impl VisionProvider for FallbackVision {
    fn model(&self) -> &str {
        self.links.first().map_or("", |l| l.model())
    }
    fn transcribe(&self, image: ImageInput) -> BoxFuture<'_, Result<ChatReply, ProviderError>> {
        Box::pin(async move { self.run(image).await.map(|r| r.reply) })
    }
    fn transcribe_routed(&self, image: ImageInput) -> BoxFuture<'_, Result<Routed, ProviderError>> {
        Box::pin(self.run(image))
    }
}
