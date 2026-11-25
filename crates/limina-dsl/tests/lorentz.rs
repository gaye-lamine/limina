//! Lorentz force magnitude bound. Expected values are computed here from
//! |q| (|E| + |v| |B|) with the numbers written out, not by the DSL.
use limina_core::{
    decide, Claim, DecisionConfig, DecisionInput, Measure, ParameterId, Quantity, Verdict,
};
use limina_dsl::Registry;
use proptest::prelude::*;
use std::collections::BTreeMap;

fn verdict(claim: f64, params: &[(&str, f64, &str)]) -> Verdict {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.get("lorentz_force").expect("law");
    let input = DecisionInput {
        claim: Claim {
            value: Measure::Plain(Quantity::from_unit(claim, "N").expect("claim unit")),
            uncertainty: None,
        },
        config: DecisionConfig::default(),
        constraints: Default::default(),
        known_parameters: params
            .iter()
            .map(|(n, v, u)| {
                (
                    ParameterId::new(*n),
                    Measure::Plain(Quantity::from_unit(*v, u).expect("parameter unit")),
                )
            })
            .collect::<BTreeMap<_, _>>(),
    };
    decide(law, &input).expect("decides").verdict
}

fn bound(q: f64, e: f64, v: f64, b: f64) -> f64 {
    q.abs() * (e.abs() + v.abs() * b.abs())
}

fn params(q: f64, e: f64, v: f64, b: f64) -> Vec<(&'static str, f64, &'static str)> {
    vec![
        ("charge", q, "C"),
        ("e_field", e, "V/m"),
        ("speed", v, "m/s"),
        ("b_field", b, "T"),
    ]
}

#[test]
fn golden_values_and_negatives() {
    // q = 1.602176634e-19 C, E = 1e5 V/m, v = 1e6 m/s, B = 0.5 T:
    // bound = 1.602176634e-19 * (1e5 + 5e5) = 9.613059804e-14 N.
    let p = params(1.602_176_634e-19, 1e5, 1e6, 0.5);
    let b = bound(1.602_176_634e-19, 1e5, 1e6, 0.5);
    assert!((b - 9.613_059_804e-14).abs() < 1e-24);
    assert_ne!(verdict(b, &p), Verdict::Impossible);
    assert_eq!(verdict(b * 0.1, &p), Verdict::Compatible);
    assert_eq!(verdict(b * 1.5, &p), Verdict::Impossible);
    // Magnetic force only: |q| v B = 1e-6 * 10 * 2 = 2e-5 N.
    let p = params(-1e-6, 0.0, 10.0, 2.0);
    assert_ne!(verdict(2e-5, &p), Verdict::Impossible);
    assert_eq!(verdict(3e-5, &p), Verdict::Impossible);
}

#[test]
fn a_missing_parameter_is_never_impossible() {
    let v = verdict(1.0, &params(1e-6, 1.0, 1.0, 1.0)[..3]);
    assert!(matches!(v, Verdict::InsufficientData { .. }), "{v:?}");
}

#[test]
fn it_has_a_classification_and_a_real_reference() {
    let registry = Registry::builtin().expect("builtin");
    let def = registry.get("lorentz_force").expect("law").definition();
    assert!(def.classification.is_some());
    assert!(def.has_real_reference());
    assert!(def.validity_domain.is_some());
    assert!(!def.limitations.is_empty());
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var("CI").is_ok() { 1024 } else { 256 },
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        ..ProptestConfig::default()
    })]
    /// A force at or below |q| (|E| + |v| |B|) is never IMPOSSIBLE.
    #[test]
    fn at_or_below_the_bound_is_never_impossible(
        q in -1e-3f64..1e-3, e in -1e6f64..1e6, v in -1e7f64..1e7, b in -10.0f64..10.0,
        frac in 0.0f64..=1.0,
    ) {
        let claim = bound(q, e, v, b) * frac;
        prop_assert_ne!(verdict(claim, &params(q, e, v, b)), Verdict::Impossible);
    }
}
