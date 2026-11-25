//! Library batch 4: quantum, atomic and nuclear physics. Expected values are
//! computed here from the textbook formulas with the constants written out
//! (CODATA 2022), not by the DSL.
use limina_core::{
    decide, Claim, DecisionConfig, DecisionInput, Measure, ParameterId, Quantity, Verdict,
};
use limina_dsl::{EqualityOutcome, Registry};
use proptest::prelude::*;
use std::collections::BTreeMap;
const C: f64 = 299_792_458.0;
const H: f64 = 6.626_070_15e-34;
const PI: f64 = std::f64::consts::PI;
const HBAR: f64 = H / (2.0 * PI);
const R_INF: f64 = 10_973_731.568_157;
const M_U: f64 = 1.660_539_068_92e-27;
const M_E: f64 = 9.109_383_713_9e-31;
fn input(claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> DecisionInput {
    DecisionInput {
        claim: Claim {
            value: Measure::Plain(Quantity::from_unit(claim, unit).expect("claim unit")),
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
    }
}
fn bound_verdict(id: &str, claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> Verdict {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.get(id).expect("law");
    decide(law, &input(claim, unit, params))
        .expect("decides")
        .verdict
}
fn evaluate(id: &str, claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> EqualityOutcome {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.equality(id).expect("law");
    law.evaluate(&input(claim, unit, params))
        .expect("evaluates")
}
fn equality_verdict(id: &str, claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> Verdict {
    match evaluate(id, claim, unit, params) {
        EqualityOutcome::Decided(d) => d.verdict,
        EqualityOutcome::OutsideModelDomain { condition } => panic!("outside: {condition}"),
    }
}
fn outside_condition(id: &str, claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> String {
    match evaluate(id, claim, unit, params) {
        EqualityOutcome::OutsideModelDomain { condition } => condition,
        other => panic!("{other:?}"),
    }
}
fn heisenberg_min_dx(dp: f64) -> f64 {
    HBAR / (2.0 * dp)
}
fn de_broglie(p: f64) -> f64 {
    H / p
}
fn hydrogen(n: f64) -> f64 {
    R_INF * H * C / (n * n)
}
fn decay(n0: f64, t: f64, t_half: f64) -> f64 {
    n0 * (-t * std::f64::consts::LN_2 / t_half).exp()
}
fn nuclear_q(m_initial: f64, m_final: f64) -> f64 {
    (m_initial - m_final).abs() * C * C
}
/// Deuterium-tritium fusion: atomic masses in u, from the atomic mass evaluation.
fn dt_masses() -> (f64, f64) {
    (
        (2.014_101_778_12 + 3.016_049_277_9) * M_U,
        (4.002_603_254_13 + 1.008_664_915_95) * M_U,
    )
}
#[test]
fn every_new_law_has_a_classification_and_a_real_reference() {
    let registry = Registry::builtin().expect("builtin");
    for id in [
        "heisenberg",
        "de_broglie",
        "hydrogen_levels",
        "radioactive_decay",
        "nuclear_q",
    ] {
        let def = registry
            .get(id)
            .map(|l| l.definition().clone())
            .or_else(|| registry.equality(id).map(|e| e.definition().clone()))
            .unwrap_or_else(|| panic!("{id} is not built in"));
        assert!(def.classification.is_some(), "{id}: classification");
        assert!(def.has_real_reference(), "{id}: reference");
        assert!(def.validity_domain.is_some(), "{id}: validity domain");
        assert!(!def.limitations.is_empty(), "{id}: limitations");
    }
}
#[test]
fn classifications_do_not_overstate_the_law() {
    let registry = Registry::builtin().expect("builtin");
    let class = |id: &str| {
        let def = registry
            .get(id)
            .map(|l| l.definition().clone())
            .or_else(|| registry.equality(id).map(|e| e.definition().clone()))
            .expect("law");
        format!("{:?}", def.classification.expect("classification"))
    };
    assert_eq!(class("heisenberg"), "Theorem");
    assert_eq!(class("hydrogen_levels"), "Model");
    assert_eq!(class("radioactive_decay"), "EmpiricalLaw");
}
/// Law id, claim unit, parameters, predicted value.
type Case = (
    &'static str,
    &'static str,
    Vec<(&'static str, f64, &'static str)>,
    f64,
);
fn cases() -> Vec<Case> {
    let (mi, mf) = dt_masses();
    vec![
        (
            "de_broglie",
            "m",
            vec![("p", M_E * 1.0e6, "kg*m/s")],
            de_broglie(M_E * 1.0e6),
        ),
        (
            "hydrogen_levels",
            "J",
            vec![("n", 2.0, "dimensionless")],
            hydrogen(2.0),
        ),
        (
            "radioactive_decay",
            "dimensionless",
            vec![
                ("n0", 1.0e6, "dimensionless"),
                ("t", 3.0, "d"),
                ("t_half", 2.0, "d"),
            ],
            decay(1.0e6, 3.0 * 86_400.0, 2.0 * 86_400.0),
        ),
        (
            "nuclear_q",
            "J",
            vec![("m_initial", mi, "kg"), ("m_final", mf, "kg")],
            nuclear_q(mi, mf),
        ),
    ]
}
#[test]
fn equality_laws_golden_values_and_negatives() {
    for (id, unit, params, predicted) in cases() {
        assert_eq!(
            equality_verdict(id, predicted, unit, &params),
            Verdict::Compatible,
            "{id}"
        );
        assert_eq!(
            equality_verdict(id, predicted * 1.0004, unit, &params),
            Verdict::Compatible,
            "{id} +0.04%"
        );
        assert_eq!(
            equality_verdict(id, predicted * 1.1, unit, &params),
            Verdict::Impossible,
            "{id} +10%"
        );
        assert_eq!(
            equality_verdict(id, predicted * 0.9, unit, &params),
            Verdict::Impossible,
            "{id} -10%"
        );
    }
}
#[test]
fn known_textbook_values() {
    // Hydrogen ground state: 13.6057 eV; first excited level: 3.4014 eV.
    let ev = 1.602_176_634e-19;
    assert!((hydrogen(1.0) / ev - 13.6057).abs() < 1e-3);
    assert!((hydrogen(2.0) / ev - 3.4014).abs() < 1e-3);
    // An electron at 1e6 m/s has a wavelength of 0.727 nm.
    assert!((de_broglie(M_E * 1.0e6) - 7.274e-10).abs() < 1e-12);
    // Confining an electron to 1e-10 m needs a momentum spread of 5.27e-25 kg m/s.
    assert!((heisenberg_min_dx(5.272_859e-25) - 1.0e-10).abs() < 1e-14);
    // After one half-life half remains; after ten, 1/1024.
    assert!((decay(1024.0, 5.0, 5.0) - 512.0).abs() < 1e-9);
    assert!((decay(1024.0, 50.0, 5.0) - 1.0).abs() < 1e-9);
    // D + T -> He-4 + n releases 17.59 MeV.
    let (mi, mf) = dt_masses();
    assert!((nuclear_q(mi, mf) / (ev * 1e6) - 17.59).abs() < 0.01);
}
#[test]
fn one_half_life_halves_the_count_through_the_engine() {
    let p = [
        ("n0", 1000.0, "dimensionless"),
        ("t", 8.0, "d"),
        ("t_half", 8.0, "d"),
    ];
    assert_eq!(
        equality_verdict("radioactive_decay", 500.0, "dimensionless", &p),
        Verdict::Compatible
    );
    assert_eq!(
        equality_verdict("radioactive_decay", 250.0, "dimensionless", &p),
        Verdict::Impossible
    );
}
#[test]
fn heisenberg_golden_and_negative() {
    let p = [("dp", 1.0e-24, "kg*m/s")];
    let bound = heisenberg_min_dx(1.0e-24);
    assert_eq!(
        bound_verdict("heisenberg", bound, "m", &p),
        Verdict::Compatible
    );
    assert_eq!(
        bound_verdict("heisenberg", 1.0, "nm", &p),
        Verdict::Compatible
    );
    assert_eq!(
        bound_verdict("heisenberg", bound * 0.9, "m", &p),
        Verdict::Impossible
    );
    // A position spread of 1e-11 m with a momentum spread of 1e-24 kg m/s
    // is inconsistent with the relation (the product is 1e-35 < hbar / 2).
    assert_eq!(
        bound_verdict("heisenberg", 1.0e-11, "m", &p),
        Verdict::Impossible
    );
}
#[test]
fn heisenberg_missing_parameter_is_insufficient_data() {
    let v = bound_verdict("heisenberg", 1.0e-11, "m", &[]);
    assert!(matches!(v, Verdict::InsufficientData { .. }), "{v:?}");
}
#[test]
fn a_missing_parameter_is_never_impossible() {
    for (id, unit, params, predicted) in cases() {
        let partial = &params[..params.len() - 1];
        if partial.is_empty() {
            continue;
        }
        let v = equality_verdict(id, predicted * 3.0, unit, partial);
        assert!(matches!(v, Verdict::InsufficientData { .. }), "{id}: {v:?}");
    }
}
#[test]
fn a_failed_validity_condition_is_outside_the_model_domain() {
    assert_eq!(
        outside_condition(
            "hydrogen_levels",
            1.0e-19,
            "J",
            &[("n", 0.5, "dimensionless")]
        ),
        "n >= 1"
    );
    assert_eq!(
        outside_condition("de_broglie", 1.0e-9, "m", &[("p", 0.0, "kg*m/s")]),
        "p > 0"
    );
    let p = [
        ("n0", 100.0, "dimensionless"),
        ("t", -1.0, "s"),
        ("t_half", 5.0, "s"),
    ];
    assert_eq!(
        outside_condition("radioactive_decay", 100.0, "dimensionless", &p),
        "t >= 0"
    );
    let p = [
        ("n0", 100.0, "dimensionless"),
        ("t", 1.0, "s"),
        ("t_half", 0.0, "s"),
    ];
    assert_eq!(
        outside_condition("radioactive_decay", 100.0, "dimensionless", &p),
        "t_half > 0"
    );
}
proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var_os("CI").is_some() { 1024 } else { 256 },
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(42),
        ..ProptestConfig::default()
    })]
    /// A position spread at or above hbar / (2 dp) is never IMPOSSIBLE.
    #[test]
    fn heisenberg_at_or_above_the_bound_is_never_impossible(
        dp in 1.0e-28f64..1.0e-20, factor in 1.0f64..1.0e6,
    ) {
        let v = bound_verdict("heisenberg", heisenberg_min_dx(dp) * factor, "m",
            &[("dp", dp, "kg*m/s")]);
        prop_assert_ne!(v, Verdict::Impossible);
    }
    /// A claim equal to the prediction is never IMPOSSIBLE, for every
    /// equality law and any parameter scale within the validity domain.
    #[test]
    fn the_prediction_is_never_impossible(scale in 0.01f64..100.0, n in 1u32..30) {
        let all: Vec<Case> = vec![
            ("de_broglie", "m", vec![("p", 1e-24 * scale, "kg*m/s")], de_broglie(1e-24 * scale)),
            ("hydrogen_levels", "J", vec![("n", f64::from(n), "dimensionless")], hydrogen(f64::from(n))),
            ("radioactive_decay", "dimensionless",
                vec![("n0", 1e6 * scale, "dimensionless"), ("t", 7.0 * scale, "s"), ("t_half", 3.0, "s")],
                decay(1e6 * scale, 7.0 * scale, 3.0)),
            ("nuclear_q", "J",
                vec![("m_initial", 5.0e-27 * scale, "kg"), ("m_final", 4.9e-27 * scale, "kg")],
                nuclear_q(5.0e-27 * scale, 4.9e-27 * scale)),
        ];
        for (id, unit, params, predicted) in all {
            let verdict = equality_verdict(id, predicted, unit, &params);
            prop_assert!(verdict != Verdict::Impossible, "{} was IMPOSSIBLE at its own prediction {}", id, predicted);
        }
    }
}
