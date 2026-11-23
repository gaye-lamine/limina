//! Live extraction against the configured provider. Ignored by default; run
//! with `cargo test -p limina-api --test analyze_live -- --ignored` when
//! `NVIDIA_API_KEY` is set. Without a key it returns at once.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use limina_api::{router_with, AppState};
use limina_extract::ExtractConfig;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn analyze(state: AppState, text: &str) -> (StatusCode, Value) {
    let request = Request::post("/v1/analyze")
        .header("content-type", "application/json")
        .body(Body::from(json!({"text": text}).to_string()))
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

#[tokio::test]
#[ignore = "calls the live provider; needs NVIDIA_API_KEY"]
async fn live_extraction_on_the_first_two_sentences() {
    let config = ExtractConfig::from_env();
    if !config.has_key() {
        return;
    }
    let state = AppState::from_config(&config).expect("state");

    let (status, body) = analyze(
        state.clone(),
        "heat engine between 600 K and 300 K at 60% efficiency",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let carnot = body["claims"]
        .as_array()
        .and_then(|c| c.iter().find(|c| c["limit_id"] == "carnot"))
        .expect("a carnot claim");
    assert_eq!(carnot["result"]["verdict"], "IMPOSSIBLE", "{carnot}");
    assert_eq!(carnot["result"]["bounds"]["permissive"]["b_max"], 0.5);

    let (status, body) = analyze(state, "power coefficient of 0.70 with a shrouded rotor").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let betz = body["claims"]
        .as_array()
        .and_then(|c| c.iter().find(|c| c["limit_id"] == "betz"))
        .expect("a betz claim");
    assert_eq!(betz["status"], "needs_input", "{betz}");
    assert_eq!(betz["reason"], "reference area not stated");
}
