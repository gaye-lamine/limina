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

/// The pre-filled text of the web app, one paragraph per request. Prints
/// the raw verdicts and traces; asserts only that no key leaks and that a
/// claim without a fundamental bound gets no verdict.
#[tokio::test]
#[ignore]
async fn the_multi_discipline_text_is_identified_from_the_library_first() {
    let file = dotenv();
    let config =
        ExtractConfig::from_lookup(|k| std::env::var(k).ok().or_else(|| file.get(k).cloned()));
    assert!(config.has_key(), "NVIDIA_API_KEY is not set");
    let key = file.get("NVIDIA_API_KEY").cloned().unwrap_or_default();
    let state = AppState::from_config(&config).expect("state");
    let paragraphs = [
        "The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.",
        "The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.",
        "The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.",
        "The Aurora heat pump delivers heat with a COP of 14.5, hot reservoir at 300 K, cold reservoir at 280 K.",
        "The Quanta logic chip erases one bit at 300 K while dissipating 1e-22 J.",
        "The Volta cell stores 5 kWh of energy in a mass of 1 kg.",
    ];
    for text in paragraphs {
        let request = Request::post("/v1/analyze")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "text": text }).to_string()))
            .expect("request");
        let response = router_with(state.clone())
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
        assert!(key.is_empty() || !body.contains(&key), "key leaked");
        println!("TEXT {text}\nHTTP {status}");
        let Ok(value) = serde_json::from_str::<Value>(&body) else {
            println!("  body: {body}");
            continue;
        };
        for claim in value["claims"].as_array().into_iter().flatten() {
            println!(
                "claim {} limit {} status {} verdict {} source {}",
                claim["quantity"],
                claim["limit_id"],
                claim["status"],
                claim["result"]["verdict"],
                claim["law"]["source"]
            );
            for step in claim["trace"].as_array().into_iter().flatten() {
                println!(
                    "  [{}] {}: {}",
                    step["status"], step["step"], step["detail"]
                );
            }
            if text.contains("Volta cell") {
                assert!(claim["result"].is_null(), "battery claim got a verdict");
            }
        }
        if value["claims"].is_null() {
            println!("  body: {body}");
        }
    }
}
