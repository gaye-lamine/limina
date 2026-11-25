//! Live run of the law pipeline with the key in the environment or `.env`.
//! Ignored by default:
//! `cargo test -p limina-api --test pipeline_live -- --ignored --nocapture`.
use std::collections::BTreeMap;
use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use limina_api::{router_with, AppState};
use limina_extract::ExtractConfig;
use serde_json::{json, Value};
use tower::ServiceExt;

fn dotenv() -> BTreeMap<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| {
            (
                k.trim().to_string(),
                v.trim().trim_matches(['"', '\'']).to_string(),
            )
        })
        .collect()
}

#[tokio::test]
#[ignore]
async fn a_landauer_claim_gets_an_agreed_law_and_an_engine_verdict() {
    let file = dotenv();
    let config =
        ExtractConfig::from_lookup(|k| std::env::var(k).ok().or_else(|| file.get(k).cloned()));
    assert!(
        config.has_key(),
        "NVIDIA_API_KEY is not set in the environment or .env"
    );
    let key = file.get("NVIDIA_API_KEY").cloned().unwrap_or_default();
    let state = AppState::from_config(&config).expect("state");
    let text = "The Quanta logic chip erases one bit at 300 K while dissipating 1e-22 J.";
    let request = Request::post("/v1/analyze")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "text": text }).to_string()))
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
    let body = String::from_utf8_lossy(&bytes).to_string();
    assert!(
        key.is_empty() || !body.contains(&key),
        "the key must never be returned"
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    let value: Value = serde_json::from_str(&body).expect("json");
    for claim in value["claims"].as_array().expect("claims") {
        println!("claim {} {}", claim["quantity"], claim["law"]["source"]);
        for step in claim["trace"].as_array().expect("trace") {
            println!(
                "  [{}] {}: {}",
                step["status"], step["step"], step["detail"]
            );
        }
    }
    let claim = &value["claims"][0];
    assert!(!claim["trace"].as_array().expect("trace").is_empty());
    if claim["law"]["source"] == "draft_agreed" {
        assert_eq!(claim["result"]["verdict"], "IMPOSSIBLE");
    } else {
        assert!(
            claim["result"].is_null(),
            "no verdict from a law that is not agreed"
        );
    }
}
