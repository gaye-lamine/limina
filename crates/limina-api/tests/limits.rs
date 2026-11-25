use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn get_limits() -> (StatusCode, Value) {
    let response = limina_api::router()
        .oneshot(
            Request::get("/v1/limits")
                .body(Body::empty())
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
    (status, serde_json::from_slice(&bytes).expect("json"))
}

#[tokio::test]
async fn lists_betz_and_carnot() {
    let (status, body) = get_limits().await;
    assert_eq!(status, StatusCode::OK);
    let limits = body["limits"].as_array().expect("limits array");
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
            "heisenberg",
            "landauer",
            "lorentz_force",
            "luminous_efficacy",
            "shannon_hartley",
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
            "time_dilation"
        ]
    );
}

#[tokio::test]
async fn betz_declares_its_assumption_and_no_parameter() {
    let (_, body) = get_limits().await;
    let betz = &body["limits"][0];
    assert_eq!(betz["direction"], "Max");
    assert_eq!(
        betz["assumptions"],
        json!([{
            "id": "reference_area",
            "options": [
                "unshrouded_rotor",
                "shrouded_total_capture_area",
                "shrouded_rotor_area_only"
            ]
        }])
    );
    assert_eq!(betz["parameters"], json!([]));
}

#[tokio::test]
async fn carnot_declares_its_temperatures() {
    let (_, body) = get_limits().await;
    let carnot = &body["limits"][1];
    assert_eq!(carnot["direction"], "Max");
    assert_eq!(
        carnot["assumptions"],
        json!([{
            "id": "cycle_model",
            "options": ["ideal_reversible_two_reservoirs"]
        }])
    );
    assert_eq!(
        carnot["parameters"],
        json!([
            {"id": "Th", "kind": "AbsoluteTemperature", "required": true, "units": ["K"]},
            {"id": "Tc", "kind": "AbsoluteTemperature", "required": true, "units": ["K"]}
        ])
    );
}

#[tokio::test]
async fn betz_claim_unit_is_dimensionless() {
    let (_, body) = get_limits().await;
    assert_eq!(body["limits"][0]["claim_unit"], "dimensionless");
}

#[tokio::test]
async fn carnot_metadata_has_claim_unit_and_parameter_units() {
    let (_, body) = get_limits().await;
    let carnot = &body["limits"][1];
    assert_eq!(carnot["claim_unit"], "dimensionless");
    assert_eq!(
        carnot["parameters"],
        json!([
            {"id": "Th", "kind": "AbsoluteTemperature", "required": true, "units": ["K"]},
            {"id": "Tc", "kind": "AbsoluteTemperature", "required": true, "units": ["K"]}
        ])
    );
}

#[tokio::test]
async fn betz_has_title_formula_and_statement() {
    let (_, body) = get_limits().await;
    let betz = &body["limits"][0];
    assert_eq!(betz["title"], "Betz limit");
    assert_eq!(betz["formula"], "Cp <= 16/27");
    assert_eq!(
        betz["statement"],
        "An ideal rotor in unconfined, incompressible flow cannot turn more than 16/27 \
         of the wind's kinetic power through its reference area into shaft power."
    );
}

#[tokio::test]
async fn carnot_has_title_formula_and_statement() {
    let (_, body) = get_limits().await;
    let carnot = &body["limits"][1];
    assert_eq!(carnot["title"], "Carnot limit");
    assert_eq!(carnot["formula"], "efficiency <= 1 - Tc/Th");
    assert_eq!(
        carnot["statement"],
        "A heat engine working between a hot reservoir at Th and a cold reservoir at Tc, \
         both in kelvin, cannot turn a larger fraction of the heat it takes in into work \
         than 1 - Tc/Th."
    );
}

#[tokio::test]
async fn cop_metadata_has_mode_temperatures_and_display_strings() {
    let (_, body) = get_limits().await;
    let cop = &body["limits"][2];
    assert_eq!(cop["id"], "cop");
    assert_eq!(cop["direction"], "Max");
    assert_eq!(cop["claim_unit"], "dimensionless");
    assert_eq!(
        cop["assumptions"],
        json!([{"id": "mode", "options": ["heating", "cooling"]}])
    );
    assert_eq!(
        cop["parameters"],
        json!([
            {"id": "Th", "kind": "AbsoluteTemperature", "required": true, "units": ["K"]},
            {"id": "Tc", "kind": "AbsoluteTemperature", "required": true, "units": ["K"]}
        ])
    );
    assert_eq!(cop["title"], "Carnot COP limit");
    assert_eq!(
        cop["formula"],
        "COP <= Th/(Th-Tc) (heating), COP <= Tc/(Th-Tc) (cooling)"
    );
    assert!(cop["statement"]
        .as_str()
        .expect("statement")
        .contains("coefficient of performance"));
}

#[tokio::test]
async fn every_limit_lists_the_display_labels_of_its_ids() {
    let (_, body) = get_limits().await;
    let betz = &body["limits"][0];
    assert_eq!(
        betz["assumption_labels"],
        json!({"reference_area": "reference area"})
    );
    assert_eq!(
        betz["option_labels"],
        json!({
            "unshrouded_rotor": "unshrouded rotor",
            "shrouded_total_capture_area": "shrouded, total capture area",
            "shrouded_rotor_area_only": "shrouded, rotor area only"
        })
    );
    assert_eq!(betz["parameter_labels"], json!({}));
    let carnot = &body["limits"][1];
    assert_eq!(
        carnot["assumption_labels"],
        json!({"cycle_model": "cycle model"})
    );
    assert_eq!(
        carnot["option_labels"],
        json!({"ideal_reversible_two_reservoirs": "ideal reversible cycle between two reservoirs"})
    );
    assert_eq!(
        carnot["parameter_labels"],
        json!({"Th": "hot reservoir temperature", "Tc": "cold reservoir temperature"})
    );
    let cop = &body["limits"][2];
    assert_eq!(cop["assumption_labels"], json!({"mode": "mode"}));
    assert_eq!(
        cop["option_labels"],
        json!({"heating": "heating", "cooling": "cooling"})
    );
    for limit in body["limits"].as_array().expect("limits") {
        for a in limit["assumptions"].as_array().expect("assumptions") {
            assert!(limit["assumption_labels"][a["id"].as_str().expect("id")].is_string());
            for o in a["options"].as_array().expect("options") {
                assert!(limit["option_labels"][o.as_str().expect("option")].is_string());
            }
        }
        for p in limit["parameters"].as_array().expect("parameters") {
            assert!(limit["parameter_labels"][p["id"].as_str().expect("id")].is_string());
        }
    }
}

#[tokio::test]
async fn display_strings_avoid_the_banned_words() {
    let (_, body) = get_limits().await;
    for limit in body["limits"].as_array().expect("limits") {
        for field in ["title", "formula", "statement"] {
            let text = limit[field]
                .as_str()
                .expect("display string")
                .to_ascii_lowercase();
            let words: Vec<&str> = text
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|w| !w.is_empty())
                .collect();
            for banned in ["fraud", "lie", "scam", "hoax"] {
                assert!(!words.contains(&banned), "{field} uses {banned}");
            }
        }
    }
}

async fn evaluate(body: Value) -> StatusCode {
    let response = limina_api::router()
        .oneshot(
            Request::post("/v1/evaluate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("valid request"),
        )
        .await
        .expect("infallible service");
    response.status()
}

#[tokio::test]
async fn every_listed_unit_is_accepted_by_evaluate() {
    let (_, body) = get_limits().await;
    for limit in body["limits"].as_array().expect("limits") {
        let id = limit["id"].as_str().expect("id");
        let claim_unit = limit["claim_unit"].as_str().expect("claim_unit");
        let mut constraints = serde_json::Map::new();
        for a in limit["assumptions"].as_array().expect("assumptions") {
            constraints.insert(
                a["id"].as_str().expect("id").to_string(),
                json!([a["options"][0]]),
            );
        }
        let status = evaluate(json!({
            "limit_id": id,
            "claim": {"value": 0.4, "unit": claim_unit},
            "constraints": constraints,
        }))
        .await;
        assert_eq!(status, StatusCode::OK, "claim unit {claim_unit} of {id}");
        for parameter in limit["parameters"].as_array().expect("parameters") {
            let name = parameter["id"].as_str().expect("parameter id");
            for unit in parameter["units"].as_array().expect("units") {
                let unit = unit.as_str().expect("unit");
                let declared = |n: &str| {
                    limit["parameters"]
                        .as_array()
                        .is_some_and(|ps| ps.iter().any(|p| p["id"] == n))
                };
                let mut parameters = serde_json::Map::new();
                if declared("Th") && declared("Tc") {
                    parameters.insert("Th".into(), json!({"value": 600.0, "unit": "K"}));
                    parameters.insert("Tc".into(), json!({"value": 300.0, "unit": "K"}));
                }
                let value = if name == "Th" { 600.0 } else { 300.0 };
                parameters.insert(name.to_string(), json!({"value": value, "unit": unit}));
                let status = evaluate(json!({
                    "limit_id": id,
                    "claim": {"value": 0.4, "unit": claim_unit},
                    "constraints": constraints,
                    "parameters": parameters
                }))
                .await;
                assert_eq!(status, StatusCode::OK, "{id}.{name} in {unit}");
            }
        }
    }
}

#[tokio::test]
async fn built_in_limits_carry_classification_references_and_validity() {
    let (_, body) = get_limits().await;
    let expected = [
        ("betz", "theoretical_bound"),
        ("carnot", "theorem"),
        ("cop", "theorem"),
    ];
    for (i, (id, classification)) in expected.iter().enumerate() {
        let limit = &body["limits"][i];
        assert_eq!(limit["id"], *id);
        assert_eq!(limit["classification"], *classification);
        let references = limit["references"].as_array().expect("references");
        assert!(!references.is_empty());
        for r in references {
            assert!(!r.as_str().expect("reference").starts_with("TODO"));
        }
        assert!(limit["validity_domain"]
            .as_str()
            .is_some_and(|v| !v.is_empty()));
        assert!(!limit["limitations"]
            .as_array()
            .expect("limitations")
            .is_empty());
    }
}
