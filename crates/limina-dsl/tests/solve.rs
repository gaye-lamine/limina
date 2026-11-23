//! Inverse questions against closed-form oracles (spec limit-author, AC 1).
//! Fixed seed; 256 cases locally, 1024 when `CI` is set. The verdicts on
//! each side are checked with the Rust limits of `limina-core`, independent
//! of the DSL.

use std::collections::{BTreeMap, BTreeSet};

use limina_core::{
    decide, AbsoluteTemperature, AssumptionId, Carnot, Claim, ConstraintsMap, Cop, DecisionConfig,
    DecisionInput, Dimension, Limit, Measure, OptionId, ParameterId, Quantity, Verdict,
};
use limina_dsl::{
    solve_for, DeclarativeLimit, Monotonicity, Registry, SolveError, SolveRequest,
    ThresholdDirection,
};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

const SEED: u64 = 0x4C49_4D49_4E41;

fn config() -> Config {
    Config {
        cases: if std::env::var_os("CI").is_some() {
            1024
        } else {
            256
        },
        rng_seed: RngSeed::Fixed(SEED),
        ..Config::default()
    }
}

fn builtin(id: &str) -> DeclarativeLimit {
    Registry::builtin()
        .expect("builtin")
        .get(id)
        .expect("limit")
        .clone()
}

fn mode(m: &str) -> ConstraintsMap {
    let mut c = ConstraintsMap::new();
    c.allow(
        AssumptionId::new("mode"),
        BTreeSet::from([OptionId::new(m)]),
    );
    c
}

fn request(claim: f64, tc: f64, constraints: ConstraintsMap) -> SolveRequest {
    SolveRequest {
        claim,
        uncertainty: None,
        constraints,
        fixed: BTreeMap::from([("Tc".to_string(), tc)]),
        free: "Th".to_string(),
        range: Some((tc * (1.0 + 1e-6), tc * 1e4)),
    }
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / a.abs().max(b.abs())
}

/// Verdict of an independent Rust limit at Th = th.
fn rust_verdict(
    limit: &dyn Limit,
    claim: f64,
    th: f64,
    tc: f64,
    constraints: ConstraintsMap,
) -> Verdict {
    let k = |v: f64| Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(v).expect("k"));
    let input = DecisionInput {
        claim: Claim {
            value: Measure::Plain(Quantity::from_si(claim, Dimension::DIMENSIONLESS).expect("q")),
            uncertainty: None,
        },
        config: DecisionConfig::default(),
        constraints,
        known_parameters: BTreeMap::from([
            (ParameterId::new("Th"), k(th)),
            (ParameterId::new("Tc"), k(tc)),
        ]),
    };
    decide(limit, &input).expect("decision").verdict
}

#[test]
fn carnot_th_for_060_at_300_k_is_750_k_at_least() {
    let s = solve_for(
        &builtin("carnot"),
        &SolveRequest {
            range: None,
            ..request(0.6, 300.0, ConstraintsMap::new())
        },
    )
    .expect("solution");
    assert!(rel(s.threshold, 750.0) < 1e-9, "{}", s.threshold);
    assert_eq!(s.direction, ThresholdDirection::AtLeast);
    assert_eq!(s.range, (400.0, 1000.0));
    assert_eq!(s.monotonicity, Monotonicity::Increasing);
    assert_eq!(s.verdict_below, Verdict::Impossible);
    assert_ne!(s.verdict_above, Verdict::Impossible);
    assert_eq!(
        s.sentence,
        "Not inconsistent with limit carnot only if Th is at least 750 K (other parameters fixed at Tc = 300 K)"
    );
    for word in ["fraud", "lie", "scam", "hoax"] {
        assert!(!s
            .sentence
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w == word));
    }
}

#[test]
fn cop_heating_and_cooling_are_at_most() {
    let h = solve_for(&builtin("cop"), &request(14.5, 280.0, mode("heating"))).expect("heating");
    assert!(rel(h.threshold, 14.5 * 280.0 / 13.5) < 1e-9);
    assert_eq!(h.direction, ThresholdDirection::AtMost);
    let c = solve_for(&builtin("cop"), &request(14.5, 280.0, mode("cooling"))).expect("cooling");
    assert!(rel(c.threshold, 280.0 * 15.5 / 14.5) < 1e-9);
    assert_eq!(c.direction, ThresholdDirection::AtMost);
    assert_eq!(c.verdict_above, Verdict::Impossible);
}

fn synthetic(formula: &str, range: Option<[f64; 2]>) -> DeclarativeLimit {
    let mut p =
        serde_json::json!({"name": "x", "dimension": "dimensionless", "units": ["dimensionless"]});
    if let Some(r) = range {
        p["test_range"] = serde_json::json!(r);
    }
    let def = serde_json::json!({
        "id": "synthetic", "title": "synthetic", "direction": "max", "bounded_dimension": "dimensionless",
        "parameters": [p], "formula": formula, "epistemic_status": "contested"
    });
    DeclarativeLimit::from_json(&def.to_string()).expect("definition")
}

fn free_x(claim: f64) -> SolveRequest {
    SolveRequest {
        claim,
        free: "x".into(),
        ..SolveRequest::default()
    }
}

#[test]
fn a_non_monotone_bound_gives_no_threshold() {
    let err = solve_for(&synthetic("(x - 2)^2 + 1", Some([0.0, 4.0])), &free_x(2.0))
        .expect_err("non-monotone");
    assert_eq!(
        err,
        SolveError::NotMonotone {
            parameter: "x".into(),
            monotonicity: Monotonicity::NonMonotone
        }
    );
    let constant =
        solve_for(&synthetic("x - x + 1", Some([0.0, 4.0])), &free_x(1.0)).expect_err("constant");
    assert!(matches!(
        constant,
        SolveError::NotMonotone {
            monotonicity: Monotonicity::Undetermined,
            ..
        }
    ));
}

#[test]
fn no_range_unknown_or_missing_parameters_and_no_crossing_are_typed_errors() {
    assert_eq!(
        solve_for(&synthetic("x", None), &free_x(1.0)),
        Err(SolveError::NoSearchRange("x".into()))
    );
    assert_eq!(
        solve_for(
            &synthetic("x", Some([0.0, 4.0])),
            &SolveRequest {
                free: "y".into(),
                ..free_x(1.0)
            }
        ),
        Err(SolveError::UnknownFreeParameter("y".into()))
    );
    let carnot = builtin("carnot");
    assert_eq!(
        solve_for(
            &carnot,
            &SolveRequest {
                claim: 0.6,
                free: "Th".into(),
                ..SolveRequest::default()
            }
        ),
        Err(SolveError::MissingFixedParameter("Tc".into()))
    );
    assert!(matches!(
        solve_for(&synthetic("x", Some([0.0, 4.0])), &free_x(9.0)),
        Err(SolveError::NoThresholdInRange { .. })
    ));
}

proptest! {
    #![proptest_config(config())]

    /// Carnot: 1 - Tc/Th = c gives Th = Tc/(1 - c).
    #[test]
    fn carnot_threshold_matches_the_closed_form(c in 0.05f64..0.95, tc in 1.0f64..1000.0) {
        let s = solve_for(&builtin("carnot"), &request(c, tc, ConstraintsMap::new())).expect("solution");
        let oracle = tc / (1.0 - c);
        prop_assert!(rel(s.threshold, oracle) < 1e-9, "c={:?} Tc={:?}: {:?} vs {:?}", c, tc, s.threshold, oracle);
        prop_assert_eq!(s.direction, ThresholdDirection::AtLeast);
        let wrong = rust_verdict(&Carnot::new(), c, oracle * (1.0 - 1e-3), tc, ConstraintsMap::new());
        let right = rust_verdict(&Carnot::new(), c, oracle * (1.0 + 1e-3), tc, ConstraintsMap::new());
        prop_assert_eq!(wrong, Verdict::Impossible, "c={:?} Tc={:?}", c, tc);
        prop_assert_ne!(right, Verdict::Impossible, "c={:?} Tc={:?}", c, tc);
        prop_assert_eq!(s.verdict_below, Verdict::Impossible);
        prop_assert_ne!(s.verdict_above, Verdict::Impossible);
    }

    /// COP heating: Th/(Th - Tc) = c gives Th = c Tc/(c - 1), for c > 1.
    #[test]
    fn cop_heating_threshold_matches_the_closed_form(c in 1.05f64..50.0, tc in 1.0f64..1000.0) {
        let s = solve_for(&builtin("cop"), &request(c, tc, mode("heating"))).expect("solution");
        let oracle = c * tc / (c - 1.0);
        prop_assert!(rel(s.threshold, oracle) < 1e-9, "c={:?} Tc={:?}: {:?} vs {:?}", c, tc, s.threshold, oracle);
        prop_assert_eq!(s.direction, ThresholdDirection::AtMost);
        let wrong = rust_verdict(&Cop::new(), c, oracle * (1.0 + 1e-3), tc, mode("heating"));
        let right = rust_verdict(&Cop::new(), c, oracle * (1.0 - 1e-3), tc, mode("heating"));
        prop_assert_eq!(wrong, Verdict::Impossible, "c={:?} Tc={:?}", c, tc);
        prop_assert_ne!(right, Verdict::Impossible, "c={:?} Tc={:?}", c, tc);
    }

    /// COP cooling: Tc/(Th - Tc) = c gives Th = Tc (1 + c)/c.
    #[test]
    fn cop_cooling_threshold_matches_the_closed_form(c in 0.05f64..50.0, tc in 1.0f64..1000.0) {
        let s = solve_for(&builtin("cop"), &request(c, tc, mode("cooling"))).expect("solution");
        let oracle = tc * (1.0 + c) / c;
        prop_assert!(rel(s.threshold, oracle) < 1e-9, "c={:?} Tc={:?}: {:?} vs {:?}", c, tc, s.threshold, oracle);
        prop_assert_eq!(s.direction, ThresholdDirection::AtMost);
        let wrong = rust_verdict(&Cop::new(), c, oracle * (1.0 + 1e-3), tc, mode("cooling"));
        let right = rust_verdict(&Cop::new(), c, oracle * (1.0 - 1e-3), tc, mode("cooling"));
        prop_assert_eq!(wrong, Verdict::Impossible, "c={:?} Tc={:?}", c, tc);
        prop_assert_ne!(right, Verdict::Impossible, "c={:?} Tc={:?}", c, tc);
    }

    /// Any non-monotone parabola gives NotMonotone and no threshold.
    #[test]
    fn a_non_monotone_synthetic_limit_is_refused(centre in 0.5f64..3.5, claim in 1.0f64..5.0) {
        let limit = synthetic(&format!("(x - {centre})^2 + 1"), Some([0.0, 4.0]));
        let r = solve_for(&limit, &free_x(claim));
        prop_assert!(matches!(r, Err(SolveError::NotMonotone { .. })), "centre {:?} claim {:?}: {:?}", centre, claim, r);
    }
}
