//! Correctif 4: one deadline for the whole `POST /v1/analyze` request. A
//! model slower than the deadline gives a clear 504 error, in plain words.
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use limina_api::{router_with, AppState, DEFAULT_ANALYZE_TIMEOUT};
use limina_extract::testing::MockProvider;
use limina_extract::{BoxFuture, ChatReply, ChatRequest, ExtractionProvider, ProviderError};
use serde_json::{json, Value};
use tower::ServiceExt;

/// A provider that answers after `delay`.
struct Slow {
    inner: MockProvider,
    delay: Duration,
}

impl ExtractionProvider for Slow {
    fn model(&self) -> &str {
        "slow-model"
    }

    fn complete(&self, request: ChatRequest) -> BoxFuture<'_, Result<ChatReply, ProviderError>> {
        Box::pin(async move {
            tokio::time::sleep(self.delay).await;
            self.inner.complete(request).await
        })
    }
}

async fn analyze(state: AppState) -> (StatusCode, Value) {
    let request = Request::post("/v1/analyze")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"text": "heat engine between 600 K and 300 K at 60% efficiency"}).to_string(),
        ))
        .expect("request");
    let response = router_with(state)
        .oneshot(request)
        .await
        .expect("infallible");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (status, serde_json::from_slice(&bytes).expect("json"))
}

fn slow_state(delay: Duration, deadline: Duration) -> AppState {
    let inner = MockProvider::new();
    inner.push_content(r#"{"claims":[],"missing":[]}"#);
    AppState::with_provider(Arc::new(Slow { inner, delay })).with_analyze_timeout(deadline)
}

#[tokio::test]
async fn an_analysis_slower_than_the_deadline_is_stopped_with_a_plain_message() {
    let (status, body) = analyze(slow_state(
        Duration::from_millis(400),
        Duration::from_millis(50),
    ))
    .await;
    assert_eq!(status, StatusCode::GATEWAY_TIMEOUT, "{body}");
    assert_eq!(body["error"]["code"], "AnalysisTimeout");
    let message = body["error"]["message"].as_str().expect("message");
    assert!(message.contains("took longer than"), "{message}");
    for word in [
        "model",
        "server",
        "API",
        "pipeline",
        "timeout",
        "extraction",
    ] {
        assert!(!message.contains(word), "{word} in {message}");
    }
}

#[tokio::test]
async fn an_analysis_within_the_deadline_answers() {
    let (status, body) = analyze(slow_state(
        Duration::from_millis(10),
        Duration::from_secs(5),
    ))
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[test]
fn the_default_deadline_is_sixty_seconds() {
    assert_eq!(DEFAULT_ANALYZE_TIMEOUT, Duration::from_secs(60));
    assert_eq!(
        AppState::default().analyze_timeout(),
        Duration::from_secs(60)
    );
}
