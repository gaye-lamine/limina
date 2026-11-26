//! Built-in laws are matched deterministically before any authoring: the
//! six illustrative paragraphs, with an extraction model that gives no
//! limit id at all (mock providers, no network).
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use limina_api::{registry, router_with, AppState};
use limina_extract::testing::{MockProvider, NamedMock};
use limina_extract::Extractor;
use serde_json::{json, Value};
use tower::ServiceExt;

struct Case {
    text: &'static str,
    quantity: &'static str,
    value: f64,
    unit: &'static str,
    evidence: &'static str,
    parameters: Value,
    law: &'static str,
    verdict: &'static str,
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            text: "The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.",
            quantity: "speed",
            value: 3.6e8,
            unit: "m/s",
            evidence: "3.6e8 m/s",
            parameters: json!({}),
            law: "speed_limit",
            verdict: "IMPOSSIBLE",
        },
        Case {
            text: "The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.",
            quantity: "position uncertainty",
            value: 1e-12,
            unit: "m",
            evidence: "within 1e-12 m",
            parameters: json!({"dp": {"value": 1e-24, "unit": "kg*m/s", "evidence": "1e-24 kg*m/s"}}),
            law: "heisenberg",
            verdict: "IMPOSSIBLE",
        },
        Case {
            text: "The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.",
            quantity: "resolution",
            value: 100.0,
            unit: "nm",
            evidence: "details of 100 nm",
            parameters: json!({
                "wavelength": {"value": 500, "unit": "nm", "evidence": "500 nm wavelength"},
                "aperture": {"value": 1.4, "unit": "dimensionless", "evidence": "numerical aperture of 1.4"}
            }),
            law: "abbe_limit",
            verdict: "CONDITIONAL",
        },
        Case {
            text: "The Aurora heat pump delivers heat with a COP of 14.5, hot reservoir at 300 K, cold reservoir at 280 K.",
            quantity: "coefficient of performance",
            value: 14.5,
            unit: "dimensionless",
            evidence: "COP of 14.5",
            parameters: json!({
                "Th": {"value": 300, "unit": "K", "evidence": "hot reservoir at 300 K"},
                "Tc": {"value": 280, "unit": "K", "evidence": "cold reservoir at 280 K"}
            }),
            law: "cop",
            verdict: "CONDITIONAL",
        },
        Case {
            text: "The Quanta logic chip erases one bit at 300 K while dissipating 1e-22 J.",
            quantity: "energy dissipated per bit erased",
            value: 1e-22,
            unit: "J",
            evidence: "dissipating 1e-22 J",
            parameters: json!({"T": {"value": 300, "unit": "K", "evidence": "one bit at 300 K"}}),
            law: "landauer",
            verdict: "IMPOSSIBLE",
        },
    ]
}

fn state() -> (Arc<MockProvider>, Arc<NamedMock>, AppState) {
    let a = Arc::new(MockProvider::new());
    let b = Arc::new(NamedMock::new("model-b"));
    let state = AppState::with_extractor(
        Extractor::new(a.clone(), registry::catalog()).with_retry_backoff(Some(Duration::ZERO)),
    )
    .with_second_model(b.clone());
    (a, b, state)
}

async fn analyze(state: &AppState, text: &str) -> Value {
    let request = Request::builder()
        .method("POST")
        .uri("/v1/analyze")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "text": text }).to_string()))
        .expect("request");
    let response = router_with(state.clone())
        .oneshot(request)
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json")
}

fn steps(claim: &Value) -> Vec<String> {
    claim["trace"]
        .as_array()
        .expect("trace")
        .iter()
        .map(|s| s["step"].as_str().expect("step").to_string())
        .collect()
}

#[tokio::test]
async fn the_five_illustrative_claims_with_a_law_use_their_built_in_law_without_authoring() {
    for case in cases() {
        let (a, b, state) = state();
        a.push_content(
            &json!({"claims": [{"limit_id": null, "quantity": case.quantity, "value": case.value,
                "unit": case.unit, "evidence": case.evidence, "constraints": {},
                "parameters": case.parameters}], "missing": []})
            .to_string(),
        );
        let body = analyze(&state, case.text).await;
        let claim = &body["claims"][0];
        assert_eq!(claim["limit_id"], case.law, "{}: {claim}", case.text);
        assert_eq!(claim["law"]["source"], "built_in", "{}", case.text);
        assert_eq!(
            claim["result"]["verdict"], case.verdict,
            "{}: {claim}",
            case.text
        );
        let names = steps(claim);
        assert!(
            !names
                .iter()
                .any(|s| s.starts_with("authoring") || s == "law"),
            "{}: {names:?}",
            case.text
        );
        // One model call (the extraction), none to the second model.
        assert_eq!(a.calls(), 1, "{}", case.text);
        assert_eq!(b.inner.calls(), 0, "{}", case.text);
        let notes: Vec<&str> = claim["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(
            notes
                .iter()
                .any(|n| n.starts_with(limina_extract::check::MATCHED_BUILT_IN)),
            "{}: {notes:?}",
            case.text
        );
    }
}

#[tokio::test]
async fn the_compound_claim_has_no_built_in_law_and_is_never_authored_as_one() {
    let (a, b, state) = state();
    a.push_content(
        &json!({"claims": [
            {"limit_id": null, "quantity": "energy stored", "value": 5, "unit": "kWh", "evidence": "5 kWh of energy", "constraints": {}, "parameters": {}},
            {"limit_id": null, "quantity": "mass", "value": 1, "unit": "kg", "evidence": "a mass of 1 kg", "constraints": {}, "parameters": {}}
        ], "missing": []})
        .to_string(),
    );
    a.push_content("No law identified.");
    let body = analyze(
        &state,
        "The Volta cell stores 5 kWh of energy in a mass of 1 kg.",
    )
    .await;
    let claims = body["claims"].as_array().expect("claims");
    assert_eq!(claims.len(), 1, "{body}");
    assert_eq!(claims[0]["quantity"], "specific energy");
    assert_eq!(claims[0]["law"]["source"], "no_law_identified");
    assert!(!steps(&claims[0]).iter().any(|s| s.starts_with("authoring")));
    assert_eq!(b.inner.calls(), 0);
}

#[tokio::test]
async fn a_hint_against_the_keywords_and_dimension_is_overridden() {
    // The model names Carnot for a speed: the dimension rules it out.
    let (a, b, state) = state();
    a.push_content(
        &json!({"claims": [{"limit_id": "carnot", "quantity": "cruise speed", "value": 3.6e8,
            "unit": "m/s", "evidence": "3.6e8 m/s", "constraints": {}, "parameters": {}}],
            "missing": []})
        .to_string(),
    );
    let body = analyze(
        &state,
        "The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.",
    )
    .await;
    assert_eq!(body["claims"][0]["limit_id"], "speed_limit", "{body}");
    assert_eq!(body["claims"][0]["result"]["verdict"], "IMPOSSIBLE");
    assert_eq!(a.calls(), 1);
    assert_eq!(b.inner.calls(), 0);
}
