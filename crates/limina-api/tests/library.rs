//! Library batch 1 through the API: listing, evaluation and analysis of a
//! claim against a built-in law (not an AI draft).
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use limina_api::{registry, router_with, AppState};
use limina_extract::testing::MockProvider;
use limina_extract::Extractor;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn call(
    state: AppState,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
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

fn plain_state() -> AppState {
    AppState::default()
}

#[tokio::test]
async fn the_library_laws_are_listed_with_classification_and_reference() {
    let (status, body) = call(plain_state(), "GET", "/v1/limits", None).await;
    assert_eq!(status, StatusCode::OK);
    let limits = body["limits"].as_array().expect("limits");
    for l in limits {
        let id = l["id"].as_str().expect("id");
        assert!(l["classification"].is_string(), "{id}: classification");
        assert!(
            !l["references"].as_array().expect("references").is_empty(),
            "{id}: references"
        );
        assert!(
            !l["title"].as_str().expect("title").is_empty(),
            "{id}: title"
        );
        for p in l["parameters"].as_array().expect("parameters") {
            assert!(
                !p["units"].as_array().expect("units").is_empty(),
                "{id}: units of {p}"
            );
        }
    }
    let speed = limits
        .iter()
        .find(|l| l["id"] == "speed_limit")
        .expect("speed_limit");
    assert_eq!(speed["direction"], "Max");
    assert_eq!(speed["classification"], "theoretical_bound");
    let gravity = limits
        .iter()
        .find(|l| l["id"] == "newton_gravity")
        .expect("newton_gravity");
    assert_eq!(gravity["direction"], "Equal");
    assert_eq!(gravity["classification"], "model");
}

#[tokio::test]
async fn evaluate_serves_a_library_bound_and_a_library_equality() {
    let (status, body) = call(
        plain_state(),
        "POST",
        "/v1/evaluate",
        Some(json!({"limit_id": "speed_limit", "claim": {"value": 3.6e8, "unit": "m/s"}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["verdict"], "IMPOSSIBLE");
    let (status, body) = call(
        plain_state(),
        "POST",
        "/v1/evaluate",
        Some(json!({
            "limit_id": "escape_velocity",
            "claim": {"value": 11.19, "unit": "km/s"},
            "parameters": {"M": {"value": 5.972e24, "unit": "kg"}, "r": {"value": 6.371e6, "unit": "m"}}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["verdict"], "COMPATIBLE");
}

#[tokio::test]
async fn a_probe_at_1_2_c_is_impossible_against_the_built_in_law() {
    let text = "The probe reaches 3.6e8 m/s, about 1.2 times the speed of light.";
    let mock = Arc::new(MockProvider::new());
    mock.push_content(
        r#"{"claims":[{"limit_id":"speed_limit","quantity":"speed of the probe","value":3.6e8,"unit":"m/s","evidence":"3.6e8 m/s","constraints":{},"parameters":{}}],"missing":[]}"#,
    );
    let state = AppState::with_extractor(
        Extractor::new(mock.clone(), registry::catalog()).with_retry_backoff(Some(Duration::ZERO)),
    );
    let (status, body) = call(state, "POST", "/v1/analyze", Some(json!({"text": text}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let claim = &body["claims"][0];
    assert_eq!(claim["limit_id"], "speed_limit");
    assert_eq!(claim["law"]["source"], "built_in", "{claim}");
    assert_eq!(claim["result"]["verdict"], "IMPOSSIBLE", "{claim}");
}
