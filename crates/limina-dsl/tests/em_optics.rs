//! Library batch 3: electromagnetism and optics. Expected values are computed
//! here from the textbook formulas with the constants written out
//! (CODATA 2022), not by the DSL.
use limina_core::{
    decide, Claim, DecisionConfig, DecisionInput, Measure, ParameterId, Quantity, Verdict,
};
use limina_dsl::{EqualityOutcome, Registry};
use proptest::prelude::*;
use std::collections::BTreeMap;
const C: f64 = 299_792_458.0;
const H: f64 = 6.626_070_15e-34;
const EPS0: f64 = 8.854_187_818_8e-12;
const KCD: f64 = 683.0;
const PI: f64 = std::f64::consts::PI;
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
fn equality_verdict(id: &str, claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> Verdict {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.equality(id).expect("law");
    match law
        .evaluate(&input(claim, unit, params))
        .expect("evaluates")
    {
        EqualityOutcome::Decided(d) => d.verdict,
        EqualityOutcome::OutsideModelDomain { condition } => panic!("outside: {condition}"),
    }
}
fn coulomb(q1: f64, q2: f64, r: f64) -> f64 {
    (q1 * q2).abs() / (4.0 * PI * EPS0 * r * r)
}
fn abbe(wavelength: f64, na: f64) -> f64 {
    wavelength / (2.0 * na)
}
fn faraday(n: f64, area: f64, db: f64, dt: f64) -> f64 {
    n * area * db.abs() / dt
}
#[test]
fn every_new_law_has_a_classification_and_a_real_reference() {
    let registry = Registry::builtin().expect("builtin");
    for id in [
        "coulomb",
        "photon_energy",
        "photon_energy_wavelength",
        "abbe_limit",
        "luminous_efficacy",
        "faraday_emf",
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
/// Law id, claim unit, parameters, predicted value.
type Case = (
    &'static str,
    &'static str,
    Vec<(&'static str, f64, &'static str)>,
    f64,
);
fn cases() -> Vec<Case> {
    vec![
        (
            "coulomb",
            "N",
            vec![("q1", 1.0, "uC"), ("q2", 2.0, "uC"), ("r", 10.0, "cm")],
            coulomb(1e-6, 2e-6, 0.1),
        ),
        (
            "photon_energy",
            "J",
            vec![("f", 540.0, "THz")],
            H * 540.0e12,
        ),
        (
            "photon_energy_wavelength",
            "J",
            vec![("wavelength", 500.0, "nm")],
            H * C / 500.0e-9,
        ),
        (
            "faraday_emf",
            "V",
            vec![
                ("n_turns", 100.0, "dimensionless"),
                ("area", 50.0, "cm^2"),
                ("dB", 200.0, "mT"),
                ("dt", 20.0, "ms"),
            ],
            faraday(100.0, 50e-4, 0.2, 0.02),
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
            equality_verdict(id, predicted * 1.002, unit, &params),
            Verdict::Compatible,
            "{id} +0.2%"
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
    // Two 1 uC and 2 uC charges 10 cm apart: 1.797 N.
    assert!((coulomb(1e-6, 2e-6, 0.1) - 1.797).abs() < 0.001);
    // A green photon at 540 THz carries 3.575e-19 J (2.23 eV).
    assert!((H * 540.0e12 - 3.5782e-19).abs() < 1e-22);
    // A 500 nm photon carries 3.973e-19 J (2.48 eV).
    assert!((H * C / 500.0e-9 - 3.9729e-19).abs() < 1e-22);
    // Abbe limit of 550 nm light with NA 1.4: 196 nm.
    assert!((abbe(550e-9, 1.4) - 196.4e-9).abs() < 0.1e-9);
    // 100 turns of 50 cm^2, 0.2 T in 20 ms: 5 V.
    assert!((faraday(100.0, 50e-4, 0.2, 0.02) - 5.0).abs() < 1e-12);
}
#[test]
fn coulomb_ignores_the_sign_of_a_charge_and_detects_a_wrong_exponent() {
    let p = [("q1", 1.0, "uC"), ("q2", -2.0, "uC"), ("r", 10.0, "cm")];
    assert_eq!(
        equality_verdict("coulomb", coulomb(1e-6, 2e-6, 0.1), "N", &p),
        Verdict::Compatible
    );
    // A force 4 times too large is what a distance of 5 cm would give.
    let p = [("q1", 1.0, "uC"), ("q2", 2.0, "uC"), ("r", 10.0, "cm")];
    assert_eq!(
        equality_verdict("coulomb", coulomb(1e-6, 2e-6, 0.05), "N", &p),
        Verdict::Impossible
    );
}
#[test]
fn abbe_limit_golden_and_negative() {
    let p = [
        ("wavelength", 550.0, "nm"),
        ("aperture", 1.4, "dimensionless"),
    ];
    let bound = abbe(550e-9, 1.4);
    assert_eq!(
        bound_verdict("abbe_limit", bound, "m", &p),
        Verdict::Compatible
    );
    assert_eq!(
        bound_verdict("abbe_limit", 250.0, "nm", &p),
        Verdict::Compatible
    );
    assert_eq!(
        bound_verdict("abbe_limit", bound * 0.9, "m", &p),
        Verdict::Impossible
    );
    // A far-field optical microscope resolving 50 nm with green light is
    // inconsistent with the limit under the far-field, linear assumptions.
    assert_eq!(
        bound_verdict("abbe_limit", 50.0, "nm", &p),
        Verdict::Impossible
    );
}
#[test]
fn abbe_limit_missing_parameter_is_insufficient_data() {
    let v = bound_verdict("abbe_limit", 50.0, "nm", &[("wavelength", 550.0, "nm")]);
    assert!(matches!(v, Verdict::InsufficientData { .. }), "{v:?}");
}
#[test]
fn luminous_efficacy_golden_and_negative() {
    assert_eq!(
        bound_verdict("luminous_efficacy", KCD, "lm/W", &[]),
        Verdict::Compatible
    );
    assert_eq!(
        bound_verdict("luminous_efficacy", 300.0, "lm/W", &[]),
        Verdict::Compatible
    );
    assert_eq!(
        bound_verdict("luminous_efficacy", 700.0, "lm/W", &[]),
        Verdict::Impossible
    );
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
fn a_non_positive_duration_or_distance_is_outside_the_model_domain() {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.equality("faraday_emf").expect("law");
    let i = input(
        1.0,
        "V",
        &[
            ("n_turns", 10.0, "dimensionless"),
            ("area", 1.0, "m^2"),
            ("dB", 1.0, "T"),
            ("dt", 0.0, "s"),
        ],
    );
    match law.evaluate(&i).expect("evaluates") {
        EqualityOutcome::OutsideModelDomain { condition } => assert_eq!(condition, "dt > 0"),
        other => panic!("{other:?}"),
    }
}
proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var_os("CI").is_some() { 1024 } else { 256 },
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(42),
        ..ProptestConfig::default()
    })]
    /// A size at or above lambda / (2 NA) is never IMPOSSIBLE.
    #[test]
    fn abbe_at_or_above_the_bound_is_never_impossible(
        wavelength in 1.0e-7f64..2.0e-6, na in 0.05f64..1.7, factor in 1.0f64..100.0,
    ) {
        let v = bound_verdict("abbe_limit", abbe(wavelength, na) * factor, "m",
            &[("wavelength", wavelength, "m"), ("aperture", na, "dimensionless")]);
        prop_assert_ne!(v, Verdict::Impossible);
    }
    /// An efficacy at or below Kcd is never IMPOSSIBLE.
    #[test]
    fn efficacy_at_or_below_kcd_is_never_impossible(frac in 0.0f64..=1.0) {
        prop_assert_ne!(bound_verdict("luminous_efficacy", KCD * frac, "lm/W", &[]), Verdict::Impossible);
    }
    /// A claim equal to the prediction is never IMPOSSIBLE, for every
    /// equality law and any parameter scale within the validity domain.
    #[test]
    fn the_prediction_is_never_impossible(scale in 0.01f64..100.0) {
        let all: Vec<Case> = vec![
            ("coulomb", "N", vec![("q1", 1e-6 * scale, "C"), ("q2", 3e-6, "C"), ("r", 0.2 * scale, "m")],
                coulomb(1e-6 * scale, 3e-6, 0.2 * scale)),
            ("photon_energy", "J", vec![("f", 1e12 * scale, "Hz")], H * 1e12 * scale),
            ("photon_energy_wavelength", "J", vec![("wavelength", 1e-7 * scale, "m")], H * C / (1e-7 * scale)),
            ("faraday_emf", "V",
                vec![("n_turns", 10.0 * scale, "dimensionless"), ("area", 0.01 * scale, "m^2"), ("dB", 0.5, "T"), ("dt", 0.1 * scale, "s")],
                faraday(10.0 * scale, 0.01 * scale, 0.5, 0.1 * scale)),
        ];
        for (id, unit, params, predicted) in all {
            let verdict = equality_verdict(id, predicted, unit, &params);
            prop_assert!(verdict != Verdict::Impossible, "{} was IMPOSSIBLE at its own prediction {}", id, predicted);
        }
    }
}
