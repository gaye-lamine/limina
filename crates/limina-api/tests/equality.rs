//! Equality laws through the API layer: the combined verdict, the wording,
//! the typed `OutsideModelDomain` refusal and the analysis status.
use std::collections::BTreeMap;

use axum::http::StatusCode;
use limina_api::analyze::{refuse, EVALUATED, NEEDS_INPUT, OUTSIDE_MODEL_DOMAIN};
use limina_api::dto::{AnalyzedClaimDto, ClaimDto, EvaluateRequest, ParameterDto};
use limina_api::error::ApiError;
use limina_api::handlers::evaluate_equality;
use limina_dsl::{EqualityLimit, LimitDefinition};

const C: f64 = 299_792_458.0;

fn kinetic() -> EqualityLimit {
    let definition: LimitDefinition = serde_json::from_str(
        r#"{"id":"kinetic","title":"Kinetic energy","direction":"equal",
        "bounded_dimension":"J","epistemic_status":"theorem","tolerance":0.01,
        "formula":"m*c^2*(1/sqrt(1 - (v/c)^2) - 1)","validity":["v < c"],
        "parameters":[
          {"name":"m","dimension":"kg","units":["kg"]},
          {"name":"v","dimension":"m/s","units":["m/s"]}]}"#,
    )
    .expect("definition");
    EqualityLimit::compile(definition).expect("compiles")
}

fn request(claim: f64, v: Option<f64>) -> EvaluateRequest {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "m".to_string(),
        ParameterDto {
            value: 1.0,
            unit: "kg".to_string(),
        },
    );
    if let Some(v) = v {
        parameters.insert(
            "v".to_string(),
            ParameterDto {
                value: v,
                unit: "m/s".to_string(),
            },
        );
    }
    EvaluateRequest {
        limit_id: "kinetic".to_string(),
        claim: ClaimDto {
            value: claim,
            unit: "J".to_string(),
            uncertainty: None,
        },
        constraints: BTreeMap::new(),
        parameters,
    }
}

/// Predicted kinetic energy of 1 kg at `beta` c, from the textbook formula
/// with c written out.
fn predicted(beta: f64) -> f64 {
    C * C * (1.0 / (1.0 - beta * beta).sqrt() - 1.0)
}

#[test]
fn the_prediction_and_its_band_are_compatible() {
    let law = kinetic();
    let e = predicted(0.6);
    for claim in [e, e * 1.005, e * 0.995] {
        let r = evaluate_equality(&law, &request(claim, Some(0.6 * C))).expect("evaluates");
        assert_eq!(r.verdict, "COMPATIBLE", "{claim}");
        let bounds = r.bounds.expect("bounds");
        assert!((bounds.permissive.b_max - e * 1.01).abs() <= e * 1e-12);
        assert!((bounds.permissive.b_min - e * 0.99).abs() <= e * 1e-12);
    }
}

#[test]
fn a_claim_outside_the_band_is_inconsistent_with_the_limit() {
    let law = kinetic();
    let e = predicted(0.6);
    for claim in [e * 1.02, e * 0.98] {
        let r = evaluate_equality(&law, &request(claim, Some(0.6 * C))).expect("evaluates");
        assert_eq!(r.verdict, "IMPOSSIBLE");
        assert_eq!(
            r.wording,
            "inconsistent with limit kinetic under assumptions none"
        );
    }
}

#[test]
fn a_missing_parameter_is_insufficient_data() {
    let r = evaluate_equality(&kinetic(), &request(1.0e16, None)).expect("evaluates");
    assert_eq!(r.verdict, "INSUFFICIENT_DATA");
    assert_eq!(r.missing_parameters[0].id, "v");
}

#[test]
fn a_failed_validity_condition_is_a_typed_refusal_not_a_verdict() {
    let err = evaluate_equality(&kinetic(), &request(1.0e17, Some(1.2 * C))).expect_err("refused");
    assert_eq!(err.code(), "OutsideModelDomain");
    assert_eq!(err.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(err.to_string().contains("v < c"), "{err}");
}

fn claim() -> AnalyzedClaimDto {
    AnalyzedClaimDto {
        limit_id: Some("kinetic".to_string()),
        quantity: "kinetic energy".to_string(),
        value: Some(1.0),
        unit: "J".to_string(),
        raw_value: 1.0,
        raw_unit: "J".to_string(),
        evidence: None,
        constraints: BTreeMap::new(),
        parameters: BTreeMap::new(),
        notes: Vec::new(),
        flags: Vec::new(),
        status: EVALUATED,
        result: None,
        reason: None,
        code: None,
        trace: Vec::new(),
        law: None,
    }
}

#[test]
fn the_analysis_status_of_a_failed_condition_is_outside_model_domain() {
    let e = evaluate_equality(&kinetic(), &request(1.0e17, Some(1.2 * C))).expect_err("refused");
    let mut dto = claim();
    refuse(&mut dto, &e);
    assert_eq!(dto.status, OUTSIDE_MODEL_DOMAIN);
    assert_eq!(dto.code, Some("OutsideModelDomain"));
    assert!(dto.reason.as_deref().is_some_and(|r| r.contains("v < c")));
    assert!(dto.result.is_none());

    let mut other = claim();
    other.status = NEEDS_INPUT;
    refuse(&mut other, &ApiError::EmptyText);
    assert_eq!(other.status, NEEDS_INPUT);
}
