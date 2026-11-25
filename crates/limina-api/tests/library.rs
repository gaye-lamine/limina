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

#[tokio::test]
async fn coulomb_force_against_distance_is_computed_by_the_engine() {
    let (status, c) = call(
        plain_state(),
        "POST",
        "/v1/curve",
        Some(json!({
            "limit_id": "coulomb",
            "free_parameter": "r",
            "parameters": {"q1": {"value": 1.0e-6, "unit": "C"}, "q2": {"value": 2.0e-6, "unit": "C"}},
            "range": [0.05, 0.5],
            "points": 10
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{c}");
    let points = c["points"].as_array().expect("points");
    assert_eq!(points.len(), 10);
    let eps0 = 8.854_187_818_8e-12;
    for p in points {
        let r = p["x"].as_f64().expect("x");
        let expected = 1.0e-6 * 2.0e-6 / (4.0 * std::f64::consts::PI * eps0 * r * r);
        let hi = p["b_max"].as_f64().expect("b_max");
        // The served curve is the upper edge of the band: prediction times (1 + tolerance).
        assert!(
            hi >= expected && hi <= expected * 1.006,
            "r = {r}: {hi} vs {expected}"
        );
    }
    let first = points[0]["b_max"].as_f64().expect("first");
    let last = points[9]["b_max"].as_f64().expect("last");
    assert!(first > last * 99.0, "inverse square: {first} vs {last}");
}

#[tokio::test]
async fn abbe_limit_against_wavelength_is_computed_by_the_engine() {
    let (status, c) = call(
        plain_state(),
        "POST",
        "/v1/curve",
        Some(json!({
            "limit_id": "abbe_limit",
            "free_parameter": "wavelength",
            "parameters": {"aperture": {"value": 1.4, "unit": "dimensionless"}},
            "range": [4.0e-7, 7.0e-7],
            "points": 7
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{c}");
    assert_eq!(c["direction"], "Min");
    for p in c["points"].as_array().expect("points") {
        let wavelength = p["x"].as_f64().expect("x");
        let expected = wavelength / (2.0 * 1.4);
        let lo = p["b_min"].as_f64().expect("b_min");
        assert!(
            (lo - expected).abs() / expected < 1e-9,
            "{lo} vs {expected}"
        );
    }
}

#[tokio::test]
async fn decay_curve_and_heisenberg_hyperbola_are_computed_by_the_engine() {
    let (status, c) = call(
        plain_state(),
        "POST",
        "/v1/curve",
        Some(json!({
            "limit_id": "radioactive_decay",
            "free_parameter": "t",
            "parameters": {"n0": {"value": 1000.0, "unit": "dimensionless"}, "t_half": {"value": 10.0, "unit": "s"}},
            "range": [0.0, 50.0],
            "points": 6
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{c}");
    for p in c["points"].as_array().expect("points") {
        let t = p["x"].as_f64().expect("x");
        let expected = 1000.0 * (-t * std::f64::consts::LN_2 / 10.0).exp();
        let hi = p["b_max"].as_f64().expect("b_max");
        assert!(
            hi >= expected && hi <= expected * 1.011,
            "t = {t}: {hi} vs {expected}"
        );
    }
    let (status, c) = call(
        plain_state(),
        "POST",
        "/v1/curve",
        Some(json!({
            "limit_id": "heisenberg",
            "free_parameter": "dp",
            "parameters": {},
            "range": [1.0e-25, 1.0e-23],
            "points": 5
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{c}");
    assert_eq!(c["direction"], "Min");
    let hbar = 6.626_070_15e-34 / (2.0 * std::f64::consts::PI);
    for p in c["points"].as_array().expect("points") {
        let dp = p["x"].as_f64().expect("x");
        let expected = hbar / (2.0 * dp);
        let lo = p["b_min"].as_f64().expect("b_min");
        assert!(
            (lo - expected).abs() / expected < 1e-9,
            "{lo} vs {expected}"
        );
    }
}
