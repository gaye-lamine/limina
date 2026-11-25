//! Library batch 5: astrophysics and cosmology. Expected values are computed
//! here from the textbook formulas with the constants written out (CODATA 2022),
//! not by the DSL.
use limina_core::{
    decide, AssumptionId, Claim, ConstraintsMap, DecisionConfig, DecisionInput, Measure, OptionId,
    ParameterId, Quantity, Verdict,
};
use limina_dsl::{EqualityOutcome, Registry};
use proptest::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
const C: f64 = 299_792_458.0;
const G: f64 = 6.674_30e-11;
const M_P: f64 = 1.672_621_925_95e-27;
const SIGMA_T: f64 = 6.652_458_705_1e-29;
const PI: f64 = std::f64::consts::PI;
/// Nominal solar mass, IAU 2015 Resolution B3 (GM_sun / G with the CODATA G).
const M_SUN: f64 = 1.988_47e30;
fn input(
    claim: f64,
    unit: &str,
    params: &[(&str, f64, &str)],
    constraints: ConstraintsMap,
) -> DecisionInput {
    DecisionInput {
        claim: Claim {
            value: Measure::Plain(Quantity::from_unit(claim, unit).expect("claim unit")),
            uncertainty: None,
        },
        config: DecisionConfig::default(),
        constraints,
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
fn eddington(m: f64) -> f64 {
    4.0 * PI * G * m * M_P * C / SIGMA_T
}
fn schwarzschild(m: f64) -> f64 {
    2.0 * G * m / (C * C)
}
fn friedmann(rho: f64) -> f64 {
    (8.0 * PI * G * rho / 3.0).sqrt()
}
fn edd_decide(
    claim: f64,
    m: f64,
    option: Option<&str>,
) -> Result<Verdict, limina_core::EngineError> {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.get("eddington_luminosity").expect("law");
    let mut constraints = ConstraintsMap::new();
    if let Some(option) = option {
        constraints.allow(
            AssumptionId::new("accretion_geometry"),
            BTreeSet::from([OptionId::new(option)]),
        );
    }
    let i = input(claim, "W", &[("M", m, "kg")], constraints);
    decide(law, &i).map(|d| d.verdict)
}
const SPHERICAL: &str = "spherical_steady_ionized_hydrogen";
fn edd_verdict(claim: f64, m: f64) -> Verdict {
    edd_decide(claim, m, Some(SPHERICAL)).expect("decides")
}
fn equality(id: &str, claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> EqualityOutcome {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.equality(id).expect("law");
    law.evaluate(&input(claim, unit, params, ConstraintsMap::new()))
        .expect("evaluates")
}
fn equality_verdict(id: &str, claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> Verdict {
    match equality(id, claim, unit, params) {
        EqualityOutcome::Decided(d) => d.verdict,
        EqualityOutcome::OutsideModelDomain { condition } => panic!("outside: {condition}"),
    }
}
#[test]
fn every_new_law_has_a_classification_and_a_real_reference() {
    let registry = Registry::builtin().expect("builtin");
    for id in [
        "eddington_luminosity",
        "schwarzschild_radius",
        "friedmann_flat",
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
    let eddington = registry.get("eddington_luminosity").expect("law");
    let class = format!("{:?}", eddington.definition().classification);
    assert_eq!(class, "Some(TheoreticalBound)");
    for id in ["schwarzschild_radius", "friedmann_flat"] {
        let def = registry.equality(id).expect("law").definition().clone();
        assert_eq!(format!("{:?}", def.classification), "Some(Model)", "{id}");
    }
}
#[test]
fn known_textbook_values() {
    // One solar mass: L_Edd = 1.26e31 W (1.26e38 erg/s).
    assert!((eddington(M_SUN) / 1.257e31 - 1.0).abs() < 2e-3);
    // One solar mass: r_s = 2.95 km.
    assert!((schwarzschild(M_SUN) - 2953.0).abs() < 1.0);
    // Critical density for H = 67.4 km/s/Mpc is 8.5e-27 kg/m^3: H follows back.
    let h = 67.4e3 / 3.085_677_581_491_367e22;
    let rho_c = 3.0 * h * h / (8.0 * PI * G);
    assert!((rho_c / 8.53e-27 - 1.0).abs() < 5e-3, "{rho_c}");
    assert!((friedmann(rho_c) / h - 1.0).abs() < 1e-12);
}
#[test]
fn the_sigma_t_constant_matches_the_classical_electron_radius() {
    // sigma_T = 8 pi r_e^2 / 3 with r_e = 2.8179403205e-15 m (CODATA 2022).
    let r_e = 2.817_940_320_5e-15;
    assert!((8.0 * PI * r_e * r_e / 3.0 / SIGMA_T - 1.0).abs() < 1e-8);
}
#[test]
fn eddington_is_decided_only_under_its_assumption() {
    let bound = eddington(M_SUN);
    assert_eq!(edd_verdict(bound * 0.5, M_SUN), Verdict::Compatible);
    // At the bound itself the uncertainty of G and sigma_T straddles the claim:\n    // the engine does not call it impossible.\n    assert_ne!(edd_verdict(bound, M_SUN), Verdict::Impossible);
    assert_eq!(edd_verdict(bound * 10.0, M_SUN), Verdict::Impossible);
    // Without the geometry, or with another one, there is no verdict.
    for option in [None, Some("non_spherical_or_other_composition")] {
        let r = edd_decide(bound * 10.0, M_SUN, option);
        assert!(
            matches!(r, Err(limina_core::EngineError::NotApplicable { .. })),
            "{option:?}: {r:?}"
        );
    }
}
#[test]
fn eddington_missing_mass_is_insufficient_data() {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.get("eddington_luminosity").expect("law");
    let i = input(1.0e40, "W", &[], ConstraintsMap::new());
    let v = decide(law, &i).expect("decides").verdict;
    assert!(matches!(v, Verdict::InsufficientData { .. }), "{v:?}");
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
            "schwarzschild_radius",
            "m",
            vec![("M", M_SUN, "kg")],
            schwarzschild(M_SUN),
        ),
        (
            "friedmann_flat",
            "Hz",
            vec![("rho", 8.5e-27, "kg/m^3")],
            friedmann(8.5e-27),
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
            equality_verdict(id, predicted * 1.001, unit, &params),
            Verdict::Compatible,
            "{id} +0.1%"
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
fn schwarzschild_accepts_grams_and_kilometres() {
    let p = [("M", M_SUN * 1.0e3, "g")];
    assert_eq!(
        equality_verdict("schwarzschild_radius", 2.953, "km", &p),
        Verdict::Compatible
    );
    assert_eq!(
        equality_verdict("schwarzschild_radius", 10.0, "km", &p),
        Verdict::Impossible
    );
}
#[test]
fn a_missing_parameter_is_never_impossible() {
    for (id, unit, _, predicted) in cases() {
        let v = equality_verdict(id, predicted * 3.0, unit, &[]);
        assert!(matches!(v, Verdict::InsufficientData { .. }), "{id}: {v:?}");
    }
}
#[test]
fn a_non_positive_density_is_outside_the_model_domain() {
    match equality("friedmann_flat", 1.0e-18, "Hz", &[("rho", 0.0, "kg/m^3")]) {
        EqualityOutcome::OutsideModelDomain { condition } => assert_eq!(condition, "rho > 0"),
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
    /// A luminosity at or below the Eddington value is never IMPOSSIBLE, and a
    /// luminosity above it gets no verdict at all without the geometry assumption.
    #[test]
    fn eddington_is_never_impossible_at_or_below_the_bound(
        m in 1.0e29f64..1.0e40, factor in 0.001f64..1.0, over in 1.0f64..1.0e6,
    ) {
        let bound = eddington(m);
        prop_assert_ne!(edd_verdict(bound * factor, m), Verdict::Impossible);
        prop_assert!(edd_decide(bound * over, m, None).is_err());
    }
    /// A claim equal to the prediction is never IMPOSSIBLE.
    #[test]
    fn the_prediction_is_never_impossible(scale in 0.01f64..100.0) {
        let all: Vec<Case> = vec![
            ("schwarzschild_radius", "m", vec![("M", 1.0e30 * scale, "kg")], schwarzschild(1.0e30 * scale)),
            ("friedmann_flat", "Hz", vec![("rho", 1.0e-26 * scale, "kg/m^3")], friedmann(1.0e-26 * scale)),
        ];
        for (id, unit, params, predicted) in all {
            let verdict = equality_verdict(id, predicted, unit, &params);
            prop_assert!(verdict != Verdict::Impossible, "{} was IMPOSSIBLE at its own prediction {}", id, predicted);
        }
    }
}
