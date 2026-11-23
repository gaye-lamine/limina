use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use proptest::prelude::*;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn post_raw(body: String) -> (StatusCode, Value) {
    let response = limina_api::router()
        .oneshot(
            Request::post("/v1/evaluate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("valid request"),
        )
        .await
        .expect("infallible service");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (status, serde_json::from_slice(&bytes).expect("json body"))
}

async fn post(body: Value) -> (StatusCode, Value) {
    post_raw(body.to_string()).await
}

fn betz(value: f64, constraints: Value) -> Value {
    json!({
        "limit_id": "betz",
        "claim": {"value": value, "unit": "dimensionless"},
        "constraints": constraints,
        "parameters": {}
    })
}

fn unshrouded() -> Value {
    json!({"reference_area": ["unshrouded_rotor"]})
}

fn carnot(value: f64, parameters: Value) -> Value {
    json!({
        "limit_id": "carnot",
        "claim": {"value": value, "unit": "dimensionless"},
        "constraints": {},
        "parameters": parameters
    })
}

fn kelvin(v: f64) -> Value {
    json!({"value": v, "unit": "K"})
}

fn cop(value: f64, constraints: Value) -> Value {
    json!({
        "limit_id": "cop",
        "claim": {"value": value, "unit": "dimensionless"},
        "constraints": constraints,
        "parameters": {"Th": kelvin(300.0), "Tc": kelvin(280.0)}
    })
}

#[tokio::test]
async fn cop_14_5_unconstrained_is_conditional_on_heating_certain() {
    let (status, body) = post(cop(14.5, json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"], "CONDITIONAL");
    assert_eq!(body["limit_id"], "cop");
    assert_eq!(
        body["configurations"],
        json!([{"assumptions": {"mode": "heating"}, "support": "Certain", "depends_on": []}])
    );
    assert_eq!(body["missing_parameters"], json!([]));
    let wording = body["wording"].as_str().expect("wording");
    assert!(wording.starts_with("possible only if"), "{wording}");
    assert!(wording.contains("mode = heating"), "{wording}");
    assert_eq!(body["bounds"]["permissive"]["b_min"], 14.0);
    assert_eq!(body["bounds"]["permissive"]["b_max"], 15.0);
}

#[tokio::test]
async fn cop_14_5_cooling_is_impossible() {
    let (status, body) = post(cop(14.5, json!({"mode": ["cooling"]}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"], "IMPOSSIBLE");
    assert_eq!(body["configurations"], json!([]));
    assert_eq!(
        body["wording"],
        "inconsistent with limit cop under assumptions mode = cooling"
    );
}

#[tokio::test]
async fn cop_14_5_heating_is_compatible() {
    let (status, body) = post(cop(14.5, json!({"mode": ["heating"]}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"], "COMPATIBLE");
}

#[tokio::test]
async fn betz_030_unshrouded_is_compatible() {
    let (status, body) = post(betz(0.30, unshrouded())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"], "COMPATIBLE");
    assert_eq!(body["limit_id"], "betz");
    assert!(body["engine_version"].as_str().is_some());
    assert_eq!(
        body["wording"],
        "does not exceed the bound; this does not prove the performance is reached"
    );
    assert_eq!(body["configurations"], json!([]));
    assert_eq!(body["missing_parameters"], json!([]));
    let b_max = body["bounds"]["restrictive"]["b_max"]
        .as_f64()
        .expect("bound");
    assert!((b_max - 16.0 / 27.0).abs() < 1e-12);
}

#[tokio::test]
async fn betz_070_unshrouded_is_impossible() {
    let (status, body) = post(betz(0.70, unshrouded())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"], "IMPOSSIBLE");
    assert_eq!(
        body["wording"],
        "inconsistent with limit betz under assumptions reference area = unshrouded rotor"
    );
    let margin = &body["bounds"]["permissive"]["margin"];
    assert!(margin["low"].as_f64().expect("margin") > 1.0);
}

#[tokio::test]
async fn betz_unconstrained_is_422_not_applicable() {
    let (status, body) = post(betz(0.30, json!({}))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "NotApplicable");
    assert!(body["error"]["message"].as_str().is_some());
    assert!(body.get("verdict").is_none());
}

#[tokio::test]
async fn carnot_060_600k_300k_is_impossible() {
    let params = json!({"Th": kelvin(600.0), "Tc": kelvin(300.0)});
    let (status, body) = post(carnot(0.60, params)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"], "IMPOSSIBLE");
    assert_eq!(
        body["wording"],
        "inconsistent with limit carnot under assumptions cycle model = ideal reversible cycle between two reservoirs"
    );
    assert_eq!(body["bounds"]["restrictive"]["b_max"], 0.5);
}

#[tokio::test]
async fn carnot_missing_temperature_is_insufficient_data() {
    let (status, body) = post(carnot(0.40, json!({"Th": kelvin(600.0)}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"], "INSUFFICIENT_DATA");
    assert_eq!(
        body["missing_parameters"],
        json!([{"id": "Tc", "status": "NoDomain"}])
    );
    assert_eq!(body["wording"], "to decide, the following is needed: Tc");
    assert!(body["bounds"].is_null());
}

#[tokio::test]
async fn carnot_th_not_above_tc_is_422() {
    let params = json!({"Th": kelvin(300.0), "Tc": kelvin(600.0)});
    let (status, body) = post(carnot(0.10, params)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "InvalidTemperatureOrder");
}

#[tokio::test]
async fn invalid_json_is_400() {
    let (status, body) = post_raw("{not json".to_string()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "InvalidJson");
}

#[tokio::test]
async fn missing_field_is_400() {
    let (status, body) = post(json!({"limit_id": "betz"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "InvalidJson");
}

#[tokio::test]
async fn unknown_limit_is_422() {
    let mut request = betz(0.3, unshrouded());
    request["limit_id"] = json!("shockley_queisser");
    let (status, body) = post(request).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "UnknownLimit");
}

#[tokio::test]
async fn unknown_unit_is_422() {
    let mut request = betz(0.3, unshrouded());
    request["claim"]["unit"] = json!("furlong");
    let (status, body) = post(request).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "UnknownUnit");
}

#[tokio::test]
async fn celsius_is_never_converted() {
    let params = json!({"Th": {"value": 600.0, "unit": "C"}, "Tc": kelvin(300.0)});
    let (status, body) = post(carnot(0.10, params)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "UnknownUnit");
}

#[tokio::test]
async fn unknown_parameter_is_422() {
    let params = json!({"Th": kelvin(600.0), "Tc": kelvin(300.0), "Tz": kelvin(1.0)});
    let (status, body) = post(carnot(0.10, params)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "UnknownParameter");
}

#[tokio::test]
async fn temperature_in_a_plain_unit_is_422() {
    let params = json!({"Th": {"value": 600.0, "unit": "W"}, "Tc": kelvin(300.0)});
    let (status, body) = post(carnot(0.10, params)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "IncompatibleDimension");
}

#[tokio::test]
async fn unknown_constraint_assumption_is_422() {
    let (status, body) = post(betz(0.3, json!({"nope": ["x"]}))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "UnknownAssumption");
}

#[tokio::test]
async fn uncertainty_widens_the_claim() {
    // 0.60 with u = 0.05 and default k = 2 gives [0.50, 0.70]: not above 0.5 + guard everywhere.
    let params = json!({"Th": kelvin(600.0), "Tc": kelvin(300.0)});
    let mut request = carnot(0.60, params);
    request["claim"]["uncertainty"] = json!(0.05);
    let (status, body) = post(request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"], "CONDITIONAL");
}

#[tokio::test]
async fn no_response_contains_a_banned_word() {
    let params = json!({"Th": kelvin(600.0), "Tc": kelvin(300.0)});
    let requests = [
        betz(0.70, unshrouded()),
        betz(0.30, unshrouded()),
        betz(0.30, json!({})),
        carnot(0.60, params),
        carnot(0.40, json!({})),
    ];
    for request in requests {
        let (_, body) = post(request).await;
        let text = body.to_string().to_ascii_lowercase();
        for word in ["fraud", "lie", "scam", "hoax"] {
            assert!(
                !text
                    .split(|c: char| !c.is_ascii_alphabetic())
                    .any(|w| w == word),
                "banned word {word} in {text}"
            );
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha, rng_seed: proptest::test_runner::RngSeed::Fixed(42), ..ProptestConfig::default() })]

    /// A claim at or below the Betz bound is never IMPOSSIBLE through the API.
    #[test]
    fn betz_claim_below_the_bound_is_never_impossible(value in 0.0f64..=(16.0 / 27.0)) {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let (status, body) = runtime.block_on(post(betz(value, unshrouded())));
        prop_assert_eq!(status, StatusCode::OK);
        prop_assert_ne!(body["verdict"].as_str().expect("verdict"), "IMPOSSIBLE");
    }

    /// A claim at or below the Carnot bound is never IMPOSSIBLE through the API.
    #[test]
    fn carnot_claim_below_the_bound_is_never_impossible(value in 0.0f64..=0.5) {
        let params = json!({"Th": kelvin(600.0), "Tc": kelvin(300.0)});
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let (status, body) = runtime.block_on(post(carnot(value, params)));
        prop_assert_eq!(status, StatusCode::OK);
        prop_assert_ne!(body["verdict"].as_str().expect("verdict"), "IMPOSSIBLE");
    }
}
