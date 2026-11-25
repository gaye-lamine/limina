//! Library batch 1: classical mechanics and relativity. Expected values are
//! computed here from the textbook formulas with the constants written out
//! (CODATA 2022), not by the DSL.
use std::collections::BTreeMap;

use limina_core::{
    decide, Claim, DecisionConfig, DecisionInput, Measure, ParameterId, Quantity, Verdict,
};
use limina_dsl::{EqualityOutcome, Registry};
use proptest::prelude::*;

const C: f64 = 299_792_458.0;
const G: f64 = 6.674_30e-11;
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

/// Law id, claim unit, parameters, predicted value.
type Case = (
    &'static str,
    &'static str,
    Vec<(&'static str, f64, &'static str)>,
    f64,
);

const NEW_LAWS: [&str; 9] = [
    "speed_limit",
    "tsiolkovsky",
    "energy_momentum",
    "escape_velocity",
    "kepler_third",
    "length_contraction",
    "newton_gravity",
    "relativistic_kinetic_energy",
    "time_dilation",
];

#[test]
fn every_new_law_has_a_classification_and_a_real_reference() {
    let registry = Registry::builtin().expect("builtin");
    for id in NEW_LAWS {
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
fn speed_limit_a_probe_at_1_2_c_is_impossible_and_below_c_is_compatible() {
    let at = |beta: f64| bound_verdict("speed_limit", beta * C, "m/s", &[]);
    assert_eq!(at(1.2), Verdict::Impossible);
    assert_eq!(at(1.0001), Verdict::Impossible);
    assert_eq!(at(0.5), Verdict::Compatible);
    assert_eq!(at(1.0), Verdict::Compatible);
}

#[test]
fn tsiolkovsky_golden_and_negative() {
    let (ve, m0, mf): (f64, f64, f64) = (3000.0, 100_000.0, 20_000.0);
    let bound = ve * (m0 / mf).ln();
    let params = [("ve", ve, "m/s"), ("m0", m0, "kg"), ("mf", mf, "kg")];
    assert!((bound - 4828.314).abs() < 1e-2);
    assert_eq!(
        bound_verdict("tsiolkovsky", bound * 0.99, "m/s", &params),
        Verdict::Compatible
    );
    assert_eq!(
        bound_verdict("tsiolkovsky", bound * 1.01, "m/s", &params),
        Verdict::Impossible
    );
    // Swapped masses give the same bound, never a false IMPOSSIBLE.
    let swapped = [("ve", ve, "m/s"), ("m0", mf, "kg"), ("mf", m0, "kg")];
    assert_eq!(
        bound_verdict("tsiolkovsky", bound * 0.99, "m/s", &swapped),
        Verdict::Compatible
    );
    // A missing parameter is INSUFFICIENT_DATA.
    let v = bound_verdict("tsiolkovsky", 1.0, "m/s", &[("ve", ve, "m/s")]);
    assert!(matches!(v, Verdict::InsufficientData { .. }));
}

/// One golden case per equality law: the prediction, a claim 10 % off.
fn cases() -> Vec<Case> {
    let (m_e, p) = (9.109_383_713_9e-31, 1.0e-22);
    let (m, v) = (2.0, 0.6 * C);
    let (mass_e, radius_e) = (5.972e24, 6.371e6);
    vec![
        (
            "energy_momentum",
            "J",
            vec![("p", p, "kg*m/s"), ("m", m_e, "kg")],
            ((p * C).powi(2) + (m_e * C * C).powi(2)).sqrt(),
        ),
        (
            "relativistic_kinetic_energy",
            "J",
            vec![("m", m, "kg"), ("v", v, "m/s")],
            m * C * C * (1.0 / (1.0 - 0.36_f64).sqrt() - 1.0),
        ),
        (
            "time_dilation",
            "s",
            vec![("tau", 10.0, "s"), ("v", v, "m/s")],
            10.0 / (1.0 - 0.36_f64).sqrt(),
        ),
        (
            "length_contraction",
            "m",
            vec![("L0", 100.0, "m"), ("v", v, "m/s")],
            100.0 * (1.0 - 0.36_f64).sqrt(),
        ),
        (
            "newton_gravity",
            "N",
            vec![
                ("m1", mass_e, "kg"),
                ("m2", 70.0, "kg"),
                ("r", radius_e, "m"),
            ],
            G * mass_e * 70.0 / (radius_e * radius_e),
        ),
        (
            "escape_velocity",
            "m/s",
            vec![("M", mass_e, "kg"), ("r", radius_e, "m")],
            (2.0 * G * mass_e / radius_e).sqrt(),
        ),
        (
            "kepler_third",
            "s",
            vec![
                ("a", 1.495_978_707e11, "m"),
                ("m1", 1.988_47e30, "kg"),
                ("m2", mass_e, "kg"),
            ],
            2.0 * PI * (1.495_978_707e11_f64.powi(3) / (G * (1.988_47e30 + mass_e))).sqrt(),
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
    // Escape velocity of the Earth: about 11.19 km/s.
    let (_, _, _, v) = cases()
        .into_iter()
        .find(|c| c.0 == "escape_velocity")
        .expect("case");
    assert!((v - 11_186.0).abs() < 20.0, "{v}");
    // Orbital period of the Earth: about one year (3.156e7 s).
    let (_, _, _, t) = cases()
        .into_iter()
        .find(|c| c.0 == "kepler_third")
        .expect("case");
    assert!((t - 3.156e7).abs() < 2.0e5, "{t}");
    // Weight of 70 kg at the surface: about 686 N.
    let (_, _, _, f) = cases()
        .into_iter()
        .find(|c| c.0 == "newton_gravity")
        .expect("case");
    assert!((f - 686.0).abs() < 5.0, "{f}");
}

#[test]
fn a_speed_at_or_above_c_is_outside_the_model_domain() {
    let registry = Registry::builtin().expect("builtin");
    for id in [
        "relativistic_kinetic_energy",
        "time_dilation",
        "length_contraction",
    ] {
        let law = registry.equality(id).expect("law");
        let (claim_unit, first) = match id {
            "relativistic_kinetic_energy" => ("J", ("m", 1.0, "kg")),
            "time_dilation" => ("s", ("tau", 1.0, "s")),
            _ => ("m", ("L0", 1.0, "m")),
        };
        let i = input(1.0, claim_unit, &[first, ("v", 1.2 * C, "m/s")]);
        match law.evaluate(&i).expect("evaluates") {
            EqualityOutcome::OutsideModelDomain { condition } => assert_eq!(condition, "v < c"),
            other => panic!("{id}: {other:?}"),
        }
    }
}

#[test]
fn a_missing_parameter_is_never_impossible() {
    for (id, unit, params, predicted) in cases() {
        let partial = &params[..params.len() - 1];
        let v = equality_verdict(id, predicted * 3.0, unit, partial);
        assert!(matches!(v, Verdict::InsufficientData { .. }), "{id}: {v:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var_os("CI").is_some() { 1024 } else { 256 },
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(42),
        ..ProptestConfig::default()
    })]

    /// A speed at or below c is never IMPOSSIBLE.
    #[test]
    fn speed_at_or_below_c_is_never_impossible(beta in 0.0f64..=1.0) {
        prop_assert_ne!(bound_verdict("speed_limit", beta * C, "m/s", &[]), Verdict::Impossible);
    }

    /// A change of speed at or below ve ln(m0/mf) is never IMPOSSIBLE.
    #[test]
    fn delta_v_at_or_below_the_bound_is_never_impossible(
        ve in 500.0f64..5000.0, mf in 100.0f64..1.0e5, ratio in 1.0f64..50.0, frac in 0.0f64..=1.0,
    ) {
        let m0 = mf * ratio;
        let bound = ve * ratio.ln();
        let v = bound_verdict("tsiolkovsky", bound * frac, "m/s",
            &[("ve", ve, "m/s"), ("m0", m0, "kg"), ("mf", mf, "kg")]);
        prop_assert_ne!(v, Verdict::Impossible);
    }

    /// A claim equal to the prediction is never IMPOSSIBLE, for every
    /// equality law and any parameter scale within the validity domain.
    #[test]
    fn the_prediction_is_never_impossible(scale in 0.01f64..100.0, beta in 0.01f64..0.99) {
        let v = beta * C;
        let all: Vec<Case> = vec![
            ("energy_momentum", "J", vec![("p", 1e-22 * scale, "kg*m/s"), ("m", 1e-30 * scale, "kg")],
                ((1e-22 * scale * C).powi(2) + (1e-30 * scale * C * C).powi(2)).sqrt()),
            ("relativistic_kinetic_energy", "J", vec![("m", scale, "kg"), ("v", v, "m/s")],
                scale * C * C * (1.0 / (1.0 - beta * beta).sqrt() - 1.0)),
            ("time_dilation", "s", vec![("tau", scale, "s"), ("v", v, "m/s")],
                scale / (1.0 - beta * beta).sqrt()),
            ("length_contraction", "m", vec![("L0", scale, "m"), ("v", v, "m/s")],
                scale * (1.0 - beta * beta).sqrt()),
            ("newton_gravity", "N", vec![("m1", 1e10 * scale, "kg"), ("m2", 1e3, "kg"), ("r", 10.0 * scale, "m")],
                G * 1e10 * scale * 1e3 / (10.0 * scale).powi(2)),
            ("escape_velocity", "m/s", vec![("M", 1e24 * scale, "kg"), ("r", 1e6 * scale, "m")],
                (2.0 * G * 1e24 * scale / (1e6 * scale)).sqrt()),
            ("kepler_third", "s", vec![("a", 1e9 * scale, "m"), ("m1", 1e30, "kg"), ("m2", 1e24 * scale, "kg")],
                2.0 * PI * ((1e9 * scale).powi(3) / (G * (1e30 + 1e24 * scale))).sqrt()),
        ];
        for (id, unit, params, predicted) in all {
            let verdict = equality_verdict(id, predicted, unit, &params);
            prop_assert!(verdict != Verdict::Impossible, "{} was IMPOSSIBLE at its own prediction {}", id, predicted);
        }
    }
}
