//! Properties of the 26/11 audit fixes, with fixed seeds: a Carnot claim
//! whose text states no heat engine cycle, and any claim against a law
//! written by AI, is never IMPOSSIBLE, whatever the numbers. A shrunk
//! counterexample prints the claim and the parameters.
use limina_api::dto::EvaluateRequest;
use limina_api::handlers::{evaluate_draft, evaluate_request};
use limina_dsl::DeclarativeLimit;
use proptest::prelude::*;
use serde_json::json;

fn request(body: serde_json::Value) -> EvaluateRequest {
    serde_json::from_value(body).expect("request")
}

fn landauer_draft() -> DeclarativeLimit {
    DeclarativeLimit::from_json(
        &json!({
            "id": "draft-landauer", "title": "Landauer", "direction": "min", "bounded_dimension": "J",
            "parameters": [{"name": "T", "kind": "absolute_temperature", "units": ["K"], "test_range": [1, 1000]}],
            "formula": "kB*T*ln2", "validity": ["T > 0"], "epistemic_status": "theorem"
        })
        .to_string(),
    )
    .expect("definition")
}

fn power_draft() -> DeclarativeLimit {
    DeclarativeLimit::from_json(
        &json!({
            "id": "draft-power", "title": "Power", "direction": "max", "bounded_dimension": "W",
            "parameters": [{"name": "P_in", "dimension": "W", "units": ["W"], "test_range": [1, 1000]}],
            "formula": "P_in", "validity": [], "epistemic_status": "theorem"
        })
        .to_string(),
    )
    .expect("definition")
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var("CI").is_ok() { 1024 } else { 256 },
        rng_seed: proptest::test_runner::RngSeed::Fixed(20261126),
        ..ProptestConfig::default()
    })]

    /// Carnot with the conversion process left open (no heat engine stated).
    #[test]
    fn carnot_without_a_stated_heat_engine_is_never_impossible(
        efficiency in 0.0f64..5.0,
        tc in 1.0f64..1000.0,
        gap in 0.1f64..2000.0,
    ) {
        let th = tc + gap;
        let r = evaluate_request(&request(json!({
            "limit_id": "carnot",
            "claim": {"value": efficiency, "unit": "dimensionless"},
            "parameters": {"Th": {"value": th, "unit": "K"}, "Tc": {"value": tc, "unit": "K"}}
        })));
        if let Ok(r) = r {
            prop_assert_ne!(r.verdict, "IMPOSSIBLE",
                "efficiency {} with Th = {} K, Tc = {} K", efficiency, th, tc);
        }
    }

    /// A law written by AI, a minimum and a maximum, even when the request
    /// asks for the formula to be taken as correct.
    #[test]
    fn a_draft_law_is_never_impossible(
        claim in 1e-30f64..1e6,
        t in 1.0f64..1000.0,
        p_in in 1.0f64..1000.0,
        insist in any::<bool>(),
    ) {
        let constraints = if insist {
            json!({"draft_formula": ["as_written"]})
        } else {
            json!({})
        };
        let landauer = evaluate_draft(&landauer_draft(), &request(json!({
            "limit_id": "draft-landauer",
            "claim": {"value": claim, "unit": "J"},
            "constraints": constraints,
            "parameters": {"T": {"value": t, "unit": "K"}}
        })));
        if let Ok(r) = landauer {
            prop_assert_ne!(r.verdict, "IMPOSSIBLE", "claim {} J at T = {} K", claim, t);
        }
        let power = evaluate_draft(&power_draft(), &request(json!({
            "limit_id": "draft-power",
            "claim": {"value": claim, "unit": "W"},
            "constraints": constraints,
            "parameters": {"P_in": {"value": p_in, "unit": "W"}}
        })));
        if let Ok(r) = power {
            prop_assert_ne!(r.verdict, "IMPOSSIBLE", "claim {} W with P_in = {} W", claim, p_in);
        }
    }
}

/// The one case the open option cannot cover is refused, not decided.
#[test]
fn a_negative_claim_against_a_draft_minimum_is_refused() {
    let r = evaluate_draft(
        &landauer_draft(),
        &request(json!({
            "limit_id": "draft-landauer",
            "claim": {"value": -1e-21, "unit": "J"},
            "parameters": {"T": {"value": 300, "unit": "K"}}
        })),
    );
    assert_eq!(r.map(|r| r.verdict).unwrap_err().code(), "DraftNotApplied");
}
