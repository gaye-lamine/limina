//! The multi-domain paragraphs of the 26/11 review and their counter-tests,
//! with extraction answers that make the mistakes seen live (a dropped unit
//! prefix, a wrong unit, a missing parameter, a duplicate claim). Mock
//! providers, no network: the verdict comes from the engine only.
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

const CAPSULE: &str = "The Atlas capsule escapes Earth's gravity from the surface with a launch speed of\nSPEED km/s and no further propulsion.";
const GENERATOR: &str = "The Perpetua generator delivers an output power of 120 W from an input power of\n100 W, its only energy source.";
const SOLAR: &str = "The Solis single-junction silicon solar cell reaches an efficiency of 45% under\nunconcentrated sunlight.";
const MODEM: &str = "The Echo modem transmits RATE Mbit/s over a 1 MHz noisy channel with a linear\nsignal-to-noise ratio of 15.";
const TELESCOPE: &str = "The Vega telescope has an angular resolution of THETA rad at a wavelength of 550\nnm with an aperture diameter of 0.10 m.";
const REACTOR: &str = "The Nova reactor releases ENERGY J of energy by converting 1 kg of mass.";
const PICO: &str = "The Pico sensor locates an electron within 1e-10 m while its momentum is known\nto within 1e-24 kg*m/s.";

fn state() -> (Arc<MockProvider>, AppState) {
    let a = Arc::new(MockProvider::new());
    let b = Arc::new(NamedMock::new("model-b"));
    let state = AppState::with_extractor(
        Extractor::new(a.clone(), registry::catalog()).with_retry_backoff(Some(Duration::ZERO)),
    )
    .with_second_model(b);
    (a, state)
}

async fn analyze(model_answer: Value, text: &str) -> Value {
    let (a, state) = state();
    a.push_content(&model_answer.to_string());
    a.push_content("No law identified.");
    let request = Request::builder()
        .method("POST")
        .uri("/v1/analyze")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "text": text }).to_string()))
        .expect("request");
    let response = router_with(state)
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

fn claim(quantity: &str, value: f64, unit: &str, evidence: &str, parameters: Value) -> Value {
    json!({"limit_id": null, "quantity": quantity, "value": value, "unit": unit,
        "evidence": evidence, "constraints": {}, "parameters": parameters})
}

fn answer(claims: Vec<Value>) -> Value {
    json!({"claims": claims, "missing": []})
}

/// The only claim of the answer, its law and its verdict.
fn single(body: &Value) -> (&Value, String, String) {
    let claims = body["claims"].as_array().expect("claims");
    assert_eq!(claims.len(), 1, "{body}");
    let c = &claims[0];
    (
        c,
        c["limit_id"].as_str().unwrap_or("").to_string(),
        c["result"]["verdict"].as_str().unwrap_or("").to_string(),
    )
}

async fn capsule(speed: &str, extra: Vec<Value>) -> (String, String) {
    let text = CAPSULE.replace("SPEED", speed);
    let v: f64 = speed.parse().expect("number");
    let mut claims = vec![claim(
        "launch speed",
        v,
        "km/s",
        &format!("{speed} km/s"),
        json!({}),
    )];
    claims.extend(extra);
    let body = analyze(answer(claims), &text).await;
    let (_, law, verdict) = single(&body);
    (law, verdict)
}

#[tokio::test]
async fn the_capsule_is_checked_against_the_escape_speed_of_earth() {
    assert_eq!(
        capsule("8", vec![]).await,
        ("escape_speed_min".into(), "IMPOSSIBLE".into())
    );
    assert_eq!(
        capsule("12", vec![]).await,
        ("escape_speed_min".into(), "COMPATIBLE".into())
    );
}

#[tokio::test]
async fn an_object_that_escapes_uses_the_minimum_speed_whatever_the_quantity_name() {
    // Seen live: the model named the quantity "escape speed".
    let text = CAPSULE.replace("SPEED", "8");
    for quantity in ["escape speed", "launch speed", "speed"] {
        let body = analyze(
            answer(vec![claim(quantity, 8.0, "km/s", "8 km/s", json!({}))]),
            &text,
        )
        .await;
        let (c, law, verdict) = single(&body);
        assert_eq!(
            (law.as_str(), verdict.as_str()),
            ("escape_speed_min", "IMPOSSIBLE"),
            "{quantity}: {c}"
        );
    }
}

#[tokio::test]
async fn the_escape_velocity_of_a_body_keeps_its_own_law() {
    let text = "The escape velocity of the planet is 11.2 km/s.";
    let body = analyze(
        answer(vec![claim(
            "escape velocity",
            11.2,
            "km/s",
            "11.2 km/s",
            json!({}),
        )]),
        text,
    )
    .await;
    let (c, law, _) = single(&body);
    assert_eq!(law, "escape_velocity", "{c}");
}

#[tokio::test]
async fn a_converted_copy_of_the_same_speed_is_one_claim() {
    // Seen live: the model gave 12 km/s and also 12000 m/s.
    let copy = claim(
        "launch speed",
        12000.0,
        "m/s",
        "launch speed of 12 km/s",
        json!({}),
    );
    assert_eq!(
        capsule("12", vec![copy]).await,
        ("escape_speed_min".into(), "COMPATIBLE".into())
    );
}

#[tokio::test]
async fn the_generator_is_one_claim_against_energy_conservation() {
    let body = analyze(
        answer(vec![
            claim(
                "output power",
                120.0,
                "W",
                "output power of 120 W",
                json!({}),
            ),
            claim(
                "input power",
                100.0,
                "W",
                "input power of\n100 W",
                json!({}),
            ),
        ]),
        GENERATOR,
    )
    .await;
    let (c, law, verdict) = single(&body);
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("power_conservation", "IMPOSSIBLE"),
        "{c}"
    );
}

#[tokio::test]
async fn the_single_junction_cell_is_checked_against_shockley_queisser() {
    let body = analyze(
        answer(vec![claim(
            "efficiency",
            45.0,
            "%",
            "efficiency of 45%",
            json!({}),
        )]),
        SOLAR,
    )
    .await;
    let (c, law, verdict) = single(&body);
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("shockley_queisser", "IMPOSSIBLE"),
        "{c}"
    );
}

async fn modem(rate: &str, b: Value) -> (String, String, f64) {
    let text = MODEM.replace("RATE", rate);
    let v: f64 = rate.parse().expect("number");
    // The model drops the prefix: "10 Mbit/s" read as 10 bit/s.
    let body = analyze(
        answer(vec![claim(
            "data rate",
            v,
            "bit/s",
            &format!("{rate} Mbit/s"),
            json!({"B": b, "snr": {"value": 15, "unit": "dimensionless", "evidence": "signal-to-noise ratio of 15"}}),
        )]),
        &text,
    )
    .await;
    let (c, law, verdict) = single(&body);
    let margin = c["result"]["bounds"]["restrictive"]["margin"]["low"]
        .as_f64()
        .unwrap_or(f64::NAN);
    (law, verdict, margin)
}

#[tokio::test]
async fn the_modem_rate_keeps_its_written_prefix() {
    let b = json!({"value": 1, "unit": "MHz", "evidence": "1 MHz"});
    let (law, verdict, margin) = modem("10", b.clone()).await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("shannon_hartley", "IMPOSSIBLE")
    );
    assert!((margin - 2.5).abs() < 1e-9, "{margin}");
    let (_, verdict, _) = modem("3", b).await;
    assert_eq!(verdict, "COMPATIBLE");
}

#[tokio::test]
async fn a_bandwidth_the_model_converted_is_read_again_from_the_text() {
    // 1 MHz given as 1e6 Hz: the value is not the written one.
    for b in [
        json!({"value": 1e6, "unit": "Hz", "evidence": "1 MHz"}),
        json!({"value": 1e6, "unit": "Hz", "evidence": "over a 1 MHz noisy channel"}),
    ] {
        let (_, verdict, _) = modem("3", b.clone()).await;
        assert_eq!(verdict, "COMPATIBLE", "{b}");
    }
}

async fn telescope(theta: &str) -> (String, String, Value) {
    let text = TELESCOPE.replace("THETA", theta);
    let v: f64 = theta.parse().expect("number");
    // The model names the unit m for an angle written in rad.
    let body = analyze(
        answer(vec![claim(
            "angular resolution",
            v,
            "m",
            &format!("{theta} rad"),
            json!({"wavelength": {"value": 550, "unit": "nm", "evidence": "550\nnm"}}),
        )]),
        &text,
    )
    .await;
    let (c, law, verdict) = single(&body);
    (law, verdict, c.clone())
}

#[tokio::test]
async fn the_telescope_is_checked_in_radians_against_rayleigh() {
    let (law, verdict, c) = telescope("1e-5").await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("rayleigh_criterion", "COMPATIBLE"),
        "{c}"
    );
    assert_eq!(c["unit"], "rad", "{c}");
    let (_, verdict, c) = telescope("5e-6").await;
    assert_eq!(verdict, "IMPOSSIBLE", "{c}");
}

async fn reactor(energy: &str) -> (String, String) {
    let text = REACTOR.replace("ENERGY", energy);
    let v: f64 = energy.parse().expect("number");
    // Seen live: one 1 kg read as both masses of another law.
    let body = analyze(
        answer(vec![claim(
            "energy released",
            v,
            "J",
            &format!("{energy} J"),
            json!({}),
        )]),
        &text,
    )
    .await;
    let (_, law, verdict) = single(&body);
    (law, verdict)
}

#[tokio::test]
async fn the_reactor_is_checked_against_mass_energy_equivalence() {
    assert_eq!(
        reactor("2e17").await,
        ("mass_energy".into(), "IMPOSSIBLE".into())
    );
    assert_eq!(
        reactor("5e16").await,
        ("mass_energy".into(), "COMPATIBLE".into())
    );
}

#[tokio::test]
async fn a_percentage_read_as_a_fraction_is_taken_as_written() {
    // Seen live: "75%" given as 0.75 with no unit.
    let text = "The Titan heat engine reaches an efficiency of 75% with a hot reservoir at\n600 K and a cold reservoir at 300 K.";
    let body = analyze(
        answer(vec![claim(
            "efficiency",
            0.75,
            "dimensionless",
            "efficiency of 75%",
            json!({"Th": {"value": 600, "unit": "K", "evidence": "hot reservoir at\n600 K"},
                   "Tc": {"value": 300, "unit": "K", "evidence": "cold reservoir at 300 K"}}),
        )]),
        text,
    )
    .await;
    let (c, law, verdict) = single(&body);
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("carnot", "IMPOSSIBLE"),
        "{c}"
    );
    assert_eq!(c["raw_unit"], "%", "{c}");
}

#[tokio::test]
async fn a_quotient_unit_the_model_read_is_kept_over_its_written_numerator() {
    // Seen in the smoke test: "5 kWh ... in a mass of 1 kg" read as 5 kWh/kg.
    let body = analyze(
        answer(vec![claim(
            "specific energy",
            5.0,
            "kWh/kg",
            "5 kWh of energy",
            json!({}),
        )]),
        "The Volta cell stores 5 kWh of energy in a mass of 1 kg.",
    )
    .await;
    let (c, _, _) = single(&body);
    assert_eq!(c["raw_unit"], "kWh/kg", "{c}");
}

/// The model's answers in order (extraction, then any law step), on `text`.
async fn analyze_seq(answers: &[String], text: &str) -> Value {
    let (a, state) = state();
    for answer in answers {
        a.push_content(answer);
    }
    let request = Request::builder()
        .method("POST")
        .uri("/v1/analyze")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "text": text }).to_string()))
        .expect("request");
    let response = router_with(state)
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

// Correctif 1 (26/11 audit): exact sentences, French and English.
const FUEL_CELL_FR: &str =
    "Cette pile à combustible atteint 65 % de rendement à 80 °C (353 K) dans un air ambiant à 20 °C (293 K)";
const FUEL_CELL_EN: &str =
    "This fuel cell reaches an efficiency of 65% at 80 °C (353 K) in ambient air at 20 °C (293 K)";

fn fuel_cell_reading(hint: Option<&str>, quantity: &str, evidence: &str) -> String {
    json!({"claims": [{"limit_id": hint, "quantity": quantity, "value": 65, "unit": "%",
        "evidence": evidence, "constraints": {},
        "parameters": {"Th": {"value": 353, "unit": "K", "evidence": "353 K"},
                       "Tc": {"value": 293, "unit": "K", "evidence": "293 K"}}}],
        "missing": []})
    .to_string()
}

/// Every reading seen or plausible for the fuel cell: Carnot hinted (as
/// live) or not, the quantity in English or French, and a naming model that
/// names Carnot from the library. None may give IMPOSSIBLE.
#[tokio::test]
async fn a_fuel_cell_at_two_temperatures_is_never_impossible() {
    let readings = [
        (FUEL_CELL_FR, "rendement", "65 %"),
        (FUEL_CELL_FR, "efficiency", "65 %"),
        (FUEL_CELL_EN, "efficiency", "65%"),
    ];
    let parameters = json!({"parameters": {
        "Th": {"value": 353, "unit": "K", "evidence": "353 K"},
        "Tc": {"value": 293, "unit": "K", "evidence": "293 K"}}})
    .to_string();
    for (text, quantity, evidence) in readings {
        for hint in [Some("carnot"), None] {
            for naming in ["No law identified.", "built-in: carnot"] {
                let body = analyze_seq(
                    &[
                        fuel_cell_reading(hint, quantity, evidence),
                        naming.to_string(),
                        parameters.clone(),
                    ],
                    text,
                )
                .await;
                for c in body["claims"].as_array().expect("claims") {
                    assert_ne!(
                        c["result"]["verdict"], "IMPOSSIBLE",
                        "{text} / {quantity} / {hint:?} / {naming}: {c}"
                    );
                    assert!(
                        c["constraints"].get("conversion_process").is_none(),
                        "a fuel cell is not a heat engine: {c}"
                    );
                }
            }
        }
    }
}

/// Two temperatures and an efficiency, with no heat engine stated: Carnot
/// applies only under that unstated assumption, so the engine answers
/// CONDITIONAL and the wording says which assumption it is.
#[tokio::test]
async fn temperatures_without_a_stated_heat_engine_give_conditional() {
    let text = "The Helix device reaches an efficiency of 75% between 600 K and 300 K.";
    let body = analyze(
        answer(vec![claim(
            "efficiency",
            75.0,
            "%",
            "efficiency of 75%",
            json!({"Th": {"value": 600, "unit": "K", "evidence": "600 K"},
                   "Tc": {"value": 300, "unit": "K", "evidence": "300 K"}}),
        )]),
        text,
    )
    .await;
    let (c, law, verdict) = single(&body);
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("carnot", "CONDITIONAL"),
        "{c}"
    );
    let wording = c["result"]["wording"].as_str().expect("wording");
    assert!(
        wording.contains("under the assumption of a heat engine cycle between two reservoirs"),
        "{wording}"
    );
}

/// The same figures in a heat engine stay IMPOSSIBLE.
#[tokio::test]
async fn a_stated_heat_engine_keeps_the_carnot_verdict() {
    let text = "The Helix heat engine reaches an efficiency of 75% between 600 K and 300 K.";
    let body = analyze(
        answer(vec![claim(
            "efficiency",
            75.0,
            "%",
            "efficiency of 75%",
            json!({"Th": {"value": 600, "unit": "K", "evidence": "600 K"},
                   "Tc": {"value": 300, "unit": "K", "evidence": "300 K"}}),
        )]),
        text,
    )
    .await;
    let (c, law, verdict) = single(&body);
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("carnot", "IMPOSSIBLE"),
        "{c}"
    );
    assert_eq!(c["constraints"]["conversion_process"], "heat_engine_cycle");
}

/// Seen live (26/11, e2e run): the wavelength of the Lumen microscope named
/// de Broglie, and the parameter pass read p = 100 kg*m/s from "details of
/// 100 nm". The unit written in the text is nm, which a momentum cannot
/// take: p is not read, and the verdict is never IMPOSSIBLE.
#[tokio::test]
async fn a_parameter_read_with_a_unit_the_text_does_not_write_is_rejected() {
    let text = "The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.";
    let reading = answer(vec![claim(
        "illumination wavelength",
        500.0,
        "nm",
        "500 nm wavelength",
        json!({}),
    )]);
    let parameters = json!({"parameters": {"p": {"value": 100, "unit": "kg*m/s",
        "evidence": "The Lumen microscope resolves details of 100 nm"}}})
    .to_string();
    let body = analyze_seq(
        &[
            reading.to_string(),
            "built-in: de_broglie".to_string(),
            parameters,
        ],
        text,
    )
    .await;
    for c in body["claims"].as_array().expect("claims") {
        assert_ne!(c["result"]["verdict"], "IMPOSSIBLE", "{c}");
        if c["limit_id"] == "de_broglie" {
            assert!(c["parameters"]["p"]["value"].is_null(), "{c}");
        }
    }
}

// Correctif 2: context and named bodies are read in the whole paragraph.

/// "Escapes" in one sentence, the speed in the next: the escape law, with
/// Earth's values, and the verdict IMPOSSIBLE (8 km/s < 11.2 km/s).
#[tokio::test]
async fn an_escape_stated_in_the_previous_sentence_selects_the_escape_speed() {
    let text = "The Atlas capsule escapes Earth's gravity from the surface. It reaches a launch speed of 8 km/s with no further propulsion.";
    for quantity in ["launch speed", "speed"] {
        let body = analyze(
            answer(vec![claim(quantity, 8.0, "km/s", "8 km/s", json!({}))]),
            text,
        )
        .await;
        let (c, law, verdict) = single(&body);
        assert_eq!(
            (law.as_str(), verdict.as_str()),
            ("escape_speed_min", "IMPOSSIBLE"),
            "{quantity}: {c}"
        );
    }
}

/// The body named in the first sentence, the escape and the speed in the
/// second: Earth's GM and radius are still used.
#[tokio::test]
async fn a_body_named_in_the_previous_sentence_gives_its_constants() {
    let text = "The Atlas capsule starts from the surface of the Earth. It escapes with a launch speed of 8 km/s and no further propulsion.";
    let body = analyze(
        answer(vec![claim(
            "launch speed",
            8.0,
            "km/s",
            "8 km/s",
            json!({}),
        )]),
        text,
    )
    .await;
    let (c, law, verdict) = single(&body);
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("escape_speed_min", "IMPOSSIBLE"),
        "{c}"
    );
    assert_eq!(c["parameters"]["GM"]["value"], 3.986004e14);
}

/// The sentence of the request, as written: it states a launch speed, not an
/// escape. Checking it against the escape speed would answer IMPOSSIBLE to
/// a claim the text does not make (a false IMPOSSIBLE), so it is checked
/// against the speed of light only and is never IMPOSSIBLE.
#[tokio::test]
async fn a_launch_speed_without_an_escape_is_not_checked_as_an_escape() {
    let text = "The Atlas capsule starts from the surface of the Earth. It reaches a launch speed of 8 km/s with no further propulsion.";
    let body = analyze(
        answer(vec![claim(
            "launch speed",
            8.0,
            "km/s",
            "8 km/s",
            json!({}),
        )]),
        text,
    )
    .await;
    let (c, law, verdict) = single(&body);
    assert_ne!(verdict, "IMPOSSIBLE", "{c}");
    assert_ne!(law, "escape_speed_min", "{c}");
}

#[tokio::test]
async fn the_pico_sensor_is_compatible_with_heisenberg() {
    let body = analyze(
        answer(vec![claim(
            "position uncertainty",
            1e-10,
            "m",
            "within 1e-10 m",
            json!({"dp": {"value": 1e-24, "unit": "kg*m/s", "evidence": "1e-24 kg*m/s"}}),
        )]),
        PICO,
    )
    .await;
    let (c, law, verdict) = single(&body);
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("heisenberg", "COMPATIBLE"),
        "{c}"
    );
}
