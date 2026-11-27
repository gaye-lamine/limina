//! `POST /v1/limits/author` and `POST /v1/solve` through the handlers, with
//! a mock provider: no network. Spec limit-author, AC 3.
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

const TEXT: &str =
    "Erasing one bit of information at temperature T dissipates at least kB T ln 2 of energy.";
const KB: f64 = 1.380649e-23;

fn state(mock: &Arc<MockProvider>) -> AppState {
    AppState::with_extractor(
        Extractor::new(mock.clone(), registry::catalog()).with_retry_backoff(Some(Duration::ZERO)),
    )
}

fn landauer() -> String {
    json!({
        "id": "landauer",
        "title": "Landauer limit",
        "direction": "min",
        "bounded_dimension": "J",
        "parameters": [{"name": "T", "kind": "absolute_temperature", "units": ["K"], "required": true, "test_range": [1, 1000]}],
        "formula": "kB*T*ln2",
        "validity": ["T > 0"],
        "epistemic_status": "theorem"
    })
    .to_string()
}

fn synthetic(formula: &str, range: Option<[f64; 2]>) -> String {
    let mut p = json!({"name": "x", "dimension": "dimensionless", "units": ["dimensionless"], "required": true});
    if let Some(r) = range {
        p["test_range"] = json!(r);
    }
    json!({
        "id": "synthetic", "title": "synthetic", "direction": "max",
        "bounded_dimension": "dimensionless", "parameters": [p],
        "formula": formula, "epistemic_status": "contested"
    })
    .to_string()
}

async fn call(
    state: &AppState,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    let request = builder
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
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
    (status, serde_json::from_slice(&bytes).expect("json"))
}

async fn author(state: &AppState, text: &str) -> (StatusCode, Value) {
    call(
        state,
        "POST",
        "/v1/limits/author",
        Some(json!({"text": text})),
    )
    .await
}

fn evaluate_body(id: &str, joules: f64) -> Value {
    json!({
        "limit_id": id,
        "claim": {"value": joules, "unit": "J"},
        "parameters": {"T": {"value": 300.0, "unit": "K"}}
    })
}

#[tokio::test]
async fn landauer_is_authored_listed_and_evaluated_as_a_draft() {
    let mock = Arc::new(MockProvider::new());
    mock.push_content(&landauer());
    let state = state(&mock);

    let (status, body) = author(&state, TEXT).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["limit_id"], "draft-landauer");
    assert_eq!(body["status"], "draft");
    assert_eq!(body["attempts"], 1);
    assert_eq!(body["errors"], json!([]));
    assert_eq!(body["definition"]["status"], "draft");
    assert_eq!(body["definition"]["references"], json!(["to be added"]));
    assert!(body["definition"]
        .get("validated_by")
        .is_none_or(Value::is_null));
    assert_eq!(body["report"]["dimension"], "passed");
    assert_eq!(body["report"]["validated"], false);

    let (status, list) = call(&state, "GET", "/v1/limits", None).await;
    assert_eq!(status, StatusCode::OK);
    let limits = list["limits"].as_array().expect("limits");
    let ids: Vec<&str> = limits
        .iter()
        .map(|l| l["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ids,
        [
            "betz",
            "carnot",
            "cop",
            "abbe_limit",
            "eddington_luminosity",
            "escape_speed_min",
            "heisenberg",
            "landauer",
            "lorentz_force",
            "luminous_efficacy",
            "mass_energy",
            "power_conservation",
            "rayleigh_criterion",
            "shannon_hartley",
            "shockley_queisser",
            "speed_limit",
            "stefan_boltzmann",
            "tsiolkovsky",
            "coulomb",
            "de_broglie",
            "energy_momentum",
            "escape_velocity",
            "faraday_emf",
            "friedmann_flat",
            "hydrogen_levels",
            "kepler_third",
            "length_contraction",
            "newton_gravity",
            "nuclear_q",
            "photon_energy",
            "photon_energy_wavelength",
            "radioactive_decay",
            "relativistic_kinetic_energy",
            "schwarzschild_radius",
            "time_dilation",
            "draft-landauer"
        ]
    );
    let built_in = limits.len() - 1;
    assert!(limits[..built_in].iter().all(|l| l.get("status").is_none()));
    assert_eq!(limits[built_in]["status"], "draft");
    assert_eq!(limits[built_in]["direction"], "Min");
    assert_eq!(limits[built_in]["formula"], "kB*T*ln2");

    let bound = KB * 300.0 * std::f64::consts::LN_2;
    let (status, ok) = call(
        &state,
        "POST",
        "/v1/evaluate",
        Some(evaluate_body("draft-landauer", 100.0 * bound)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ok}");
    assert_eq!(ok["verdict"], "COMPATIBLE");
    assert!(ok["wording"]
        .as_str()
        .expect("wording")
        .starts_with("draft limit draft-landauer: "));
    // The decisive bound of a minimum is the largest: the AI-written one
    // (the open formula status bounds nothing, at 0).
    let b = ok["bounds"]["permissive"]["b_max"].as_f64().expect("b_max");
    assert!((b - bound).abs() <= 1e-9 * bound, "{b} vs {bound}");

    let (status, low) = call(
        &state,
        "POST",
        "/v1/evaluate",
        Some(evaluate_body("draft-landauer", bound / 3.0)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // An unvalidated AI-written law never gives IMPOSSIBLE: CONDITIONAL,
    // possible only if its formula is not taken as correct.
    assert_eq!(low["verdict"], "CONDITIONAL");
    assert_eq!(
        low["configurations"][0]["assumptions"]["draft_formula"],
        "not_validated"
    );
    assert_eq!(
        low["wording"],
        "inconsistent with draft limit draft-landauer if its formula is correct; the formula is an unvalidated draft written by AI, so no IMPOSSIBLE verdict is given"
    );
    for word in ["fraud", "lie", "scam", "hoax"] {
        assert!(!low["wording"].as_str().expect("wording").contains(word));
    }
}

#[tokio::test]
async fn built_in_evaluation_is_unchanged_with_drafts_registered() {
    let mock = Arc::new(MockProvider::new());
    mock.push_content(&landauer());
    let state = state(&mock);
    author(&state, TEXT).await;
    let body = json!({
        "limit_id": "carnot",
        "claim": {"value": 0.6, "unit": "dimensionless"},
        "constraints": {"conversion_process": ["heat_engine_cycle"]},
        "parameters": {"Th": {"value": 600.0, "unit": "K"}, "Tc": {"value": 300.0, "unit": "K"}}
    });
    let (status, r) = call(&state, "POST", "/v1/evaluate", Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(r["verdict"], "IMPOSSIBLE");
    assert!(r["wording"]
        .as_str()
        .expect("wording")
        .starts_with("inconsistent with limit carnot under assumptions "));
}

#[tokio::test]
async fn carnot_hot_reservoir_for_sixty_percent_is_750_kelvin() {
    let state = AppState::default();
    let body = json!({
        "limit_id": "carnot",
        "claim": {"value": 0.6, "unit": "dimensionless"},
        "parameters": {"Tc": {"value": 300.0, "unit": "K"}},
        "free_parameter": "Th"
    });
    let (status, r) = call(&state, "POST", "/v1/solve", Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    let th = r["threshold"].as_f64().expect("threshold");
    assert!((th - 750.0).abs() <= 1e-9 * 750.0, "{th}");
    assert_eq!(r["direction"], "at least");
    assert_eq!(r["unit"], "K");
    assert_eq!(r["monotonicity"], "Increasing");
    assert_eq!(r["verdict_below"], "IMPOSSIBLE");
    assert_eq!(r["verdict_above"], "COMPATIBLE");
    let sentence = r["sentence"].as_str().expect("sentence");
    assert!(sentence.starts_with("Not inconsistent with limit carnot only if Th is at least "));
}

#[tokio::test]
async fn landauer_draft_temperature_threshold_is_at_most() {
    let mock = Arc::new(MockProvider::new());
    mock.push_content(&landauer());
    let state = state(&mock);
    author(&state, TEXT).await;
    let body = json!({
        "limit_id": "draft-landauer",
        "claim": {"value": 5e-21, "unit": "J"},
        "free_parameter": "T"
    });
    let (status, r) = call(&state, "POST", "/v1/solve", Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    let expected = 5e-21 / (KB * std::f64::consts::LN_2);
    let t = r["threshold"].as_f64().expect("threshold");
    assert!((t - expected).abs() <= 1e-9 * expected, "{t} vs {expected}");
    assert_eq!(r["direction"], "at most");
    assert_eq!(r["verdict_below"], "COMPATIBLE");
    // Above the threshold the claim is beyond the AI-written bound: CONDITIONAL, never IMPOSSIBLE.
    assert_eq!(r["verdict_above"], "CONDITIONAL");
    assert!(r["sentence"]
        .as_str()
        .expect("sentence")
        .starts_with("Not inconsistent with draft limit draft-landauer only if T is at most "));
}

async fn solve_synthetic(formula: &str, range: Option<[f64; 2]>) -> (StatusCode, Value) {
    let mock = Arc::new(MockProvider::new());
    mock.push_content(&synthetic(formula, range));
    let state = state(&mock);
    let (status, body) = author(&state, "a synthetic limit").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let body = json!({
        "limit_id": "draft-synthetic",
        "claim": {"value": 2.0, "unit": "dimensionless"},
        "free_parameter": "x"
    });
    call(&state, "POST", "/v1/solve", Some(body)).await
}

#[tokio::test]
async fn a_non_monotone_bound_is_422_not_monotone() {
    let (status, r) = solve_synthetic("(x - 2)^2 + 1", Some([0.0, 4.0])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r["error"]["code"], "NotMonotone");
}

#[tokio::test]
async fn no_range_is_422_no_search_range() {
    let (status, r) = solve_synthetic("x + 1", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r["error"]["code"], "NoSearchRange");
}

#[tokio::test]
async fn solve_errors_have_stable_codes() {
    let state = AppState::default();
    let cases = [
        (
            json!({"limit_id": "carnot", "claim": {"value": 0.6, "unit": "dimensionless"}, "free_parameter": "Th"}),
            "MissingFixedParameter",
        ),
        (
            json!({"limit_id": "carnot", "claim": {"value": 0.6, "unit": "dimensionless"}, "parameters": {"Tc": {"value": 300.0, "unit": "K"}}, "free_parameter": "Tx"}),
            "UnknownFreeParameter",
        ),
        (
            json!({"limit_id": "carnot", "claim": {"value": 0.99, "unit": "dimensionless"}, "parameters": {"Tc": {"value": 300.0, "unit": "K"}}, "free_parameter": "Th"}),
            "NoThresholdInRange",
        ),
        (
            json!({"limit_id": "nope", "claim": {"value": 0.6, "unit": "dimensionless"}, "free_parameter": "Th"}),
            "UnknownLimit",
        ),
    ];
    for (body, code) in cases {
        let (status, r) = call(&state, "POST", "/v1/solve", Some(body)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{code}: {r}");
        assert_eq!(r["error"]["code"], code);
    }
}

#[tokio::test]
async fn authoring_without_a_key_is_503() {
    let (status, r) = author(&AppState::default(), TEXT).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(r["error"]["code"], "ExtractionUnavailable");
}

#[tokio::test]
async fn only_the_fifty_newest_drafts_stay_in_memory() {
    let state = AppState::default();
    for i in 0..52 {
        let mut v: Value = serde_json::from_str(&landauer()).expect("json");
        v["id"] = json!(format!("draft-law-{i:02}"));
        v["status"] = json!("draft");
        v["references"] = json!(["to be added"]);
        let definition: limina_dsl::LimitDefinition =
            serde_json::from_value(v).expect("definition");
        state.register(limina_dsl::DeclarativeLimit::compile(definition).expect("compiles"));
    }
    assert_eq!(
        state.authored_limits().len(),
        limina_api::MAX_AUTHORED_LIMITS
    );
    let (status, r) = call(&state, "GET", "/v1/limits", None).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = r["limits"]
        .as_array()
        .expect("limits")
        .iter()
        .filter(|l| l["status"] == "draft")
        .map(|l| l["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids.len(), limina_api::MAX_AUTHORED_LIMITS);
    assert!(!ids.contains(&"draft-law-00") && !ids.contains(&"draft-law-01"));
    assert!(ids.contains(&"draft-law-02") && ids.contains(&"draft-law-51"));
    // An evicted draft is gone for evaluation too; a kept one still answers.
    let (gone, _) = call(
        &state,
        "POST",
        "/v1/evaluate",
        Some(evaluate_body("draft-law-00", 1e-21)),
    )
    .await;
    assert_eq!(gone, StatusCode::UNPROCESSABLE_ENTITY);
    let (kept, _) = call(
        &state,
        "POST",
        "/v1/evaluate",
        Some(evaluate_body("draft-law-51", 1e-21)),
    )
    .await;
    assert_eq!(kept, StatusCode::OK);
}

#[tokio::test]
async fn a_plain_parameter_given_a_wrong_kind_is_authored_at_the_first_attempt() {
    let mock = Arc::new(MockProvider::new());
    let state = state(&mock);
    let mut v: Value = serde_json::from_str(&synthetic("x", Some([1.0, 10.0]))).expect("json");
    v["parameters"][0]["kind"] = json!("dimension");
    mock.push_content(&v.to_string());
    let (status, r) = author(&state, TEXT).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    assert_eq!(r["attempts"], 1);
    assert!(r["overrides"].to_string().contains("kind"), "{r}");
}

#[tokio::test]
async fn authoring_text_is_at_most_2000_characters() {
    let mock = Arc::new(MockProvider::new());
    let state = state(&mock);
    let (status, r) = author(&state, &"é".repeat(2001)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r["error"]["code"], "TextTooLong");
    let (status, r) = author(&state, "   ").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r["error"]["code"], "EmptyText");
    assert_eq!(mock.calls(), 0);
}

#[tokio::test]
async fn three_refused_answers_are_422_and_register_nothing() {
    let mock = Arc::new(MockProvider::new());
    for _ in 0..3 {
        mock.push_content("not a definition");
    }
    let state = state(&mock);
    let (status, r) = author(&state, TEXT).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert_eq!(r["error"]["code"], "LimitNotAuthored");
    assert_eq!(mock.calls(), 3);
    let (_, list) = call(&state, "GET", "/v1/limits", None).await;
    assert_eq!(list["limits"].as_array().expect("limits").len(), 35);
}
