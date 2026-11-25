//! Library batch 2: thermodynamics, statistical physics and information.
//! Expected values are computed here from the textbook formulas with the
//! constants written out (CODATA 2022), not by the DSL.
use limina_core::{
    decide, AbsoluteTemperature, Claim, DecisionConfig, DecisionInput, Measure, ParameterId,
    Quantity, Verdict,
};
use limina_dsl::Registry;
use proptest::prelude::*;
use std::collections::BTreeMap;
const KB: f64 = 1.380_649e-23;
const SIGMA_SB: f64 = 5.670_374_419e-8;
fn verdict(id: &str, claim: f64, unit: &str, params: &[(&str, f64, &str)]) -> Verdict {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.get(id).expect("law");
    let input = DecisionInput {
        claim: Claim {
            value: Measure::Plain(Quantity::from_unit(claim, unit).expect("claim unit")),
            uncertainty: None,
        },
        config: DecisionConfig::default(),
        constraints: Default::default(),
        known_parameters: params
            .iter()
            .map(|(n, v, u)| (ParameterId::new(*n), parameter(*v, u)))
            .collect::<BTreeMap<_, _>>(),
    };
    decide(law, &input).expect("decides").verdict
}
fn parameter(v: f64, unit: &str) -> Measure {
    if unit == "K" {
        Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(v).expect("kelvin"))
    } else {
        Measure::Plain(Quantity::from_unit(v, unit).expect("parameter unit"))
    }
}
fn shannon(b_hz: f64, snr: f64) -> f64 {
    b_hz * (1.0 + snr).log2()
}
#[test]
fn the_new_laws_have_a_classification_and_a_real_reference() {
    let registry = Registry::builtin().expect("builtin");
    for id in ["landauer", "stefan_boltzmann", "shannon_hartley"] {
        let def = registry.get(id).expect(id).definition().clone();
        assert!(def.classification.is_some(), "{id}: classification");
        assert!(def.has_real_reference(), "{id}: reference");
        assert!(def.validity_domain.is_some(), "{id}: validity domain");
        assert!(!def.limitations.is_empty(), "{id}: limitations");
    }
}
#[test]
fn landauer_golden_and_negative() {
    let bound = KB * 300.0 * std::f64::consts::LN_2;
    assert!((bound - 2.870_978e-21).abs() < 1e-26, "{bound}");
    let t = [("T", 300.0, "K")];
    assert_eq!(verdict("landauer", bound, "J", &t), Verdict::Compatible);
    assert_eq!(
        verdict("landauer", bound * 1e6, "J", &t),
        Verdict::Compatible
    );
    assert_eq!(
        verdict("landauer", bound * 0.5, "J", &t),
        Verdict::Impossible
    );
    // A claim of 1e-25 J per erased bit at 300 K is inconsistent with the limit.
    assert_eq!(verdict("landauer", 1.0e-25, "J", &t), Verdict::Impossible);
}
#[test]
fn landauer_missing_temperature_is_insufficient_data() {
    let v = verdict("landauer", 1.0e-25, "J", &[]);
    assert!(matches!(v, Verdict::InsufficientData { .. }), "{v:?}");
}
#[test]
fn stefan_boltzmann_golden_and_negative() {
    // The Sun's photosphere, 5772 K: about 6.29e7 W/m^2.
    let flux = SIGMA_SB * 5772.0_f64.powi(4);
    assert!((flux - 6.29e7).abs() < 0.02e7, "{flux}");
    let t = [("T", 5772.0, "K")];
    assert_eq!(
        verdict("stefan_boltzmann", flux, "W/m^2", &t),
        Verdict::Compatible
    );
    assert_eq!(
        verdict("stefan_boltzmann", flux * 0.5, "W/m^2", &t),
        Verdict::Compatible
    );
    assert_eq!(
        verdict("stefan_boltzmann", flux * 1.01, "W/m^2", &t),
        Verdict::Impossible
    );
    // Room temperature: a surface at 300 K cannot emit 1000 W/m^2 (bound 459 W/m^2).
    let room = SIGMA_SB * 300.0_f64.powi(4);
    assert!((room - 459.3).abs() < 0.1, "{room}");
    assert_eq!(
        verdict("stefan_boltzmann", 1000.0, "W/m^2", &[("T", 300.0, "K")]),
        Verdict::Impossible
    );
}
#[test]
fn stefan_boltzmann_missing_temperature_is_insufficient_data() {
    let v = verdict("stefan_boltzmann", 1.0e9, "W/m^2", &[]);
    assert!(matches!(v, Verdict::InsufficientData { .. }), "{v:?}");
}
#[test]
fn shannon_hartley_golden_and_negative() {
    // 1 MHz at a linear SNR of 1000 (30 dB): about 9.967 Mbit/s.
    let c = shannon(1.0e6, 1000.0);
    assert!((c - 9.967e6).abs() < 1.0e3, "{c}");
    let p = [("B", 1.0, "MHz"), ("snr", 1000.0, "dimensionless")];
    assert_eq!(
        verdict("shannon_hartley", c, "bit/s", &p),
        Verdict::Compatible
    );
    assert_eq!(
        verdict("shannon_hartley", 5.0e6, "bit/s", &p),
        Verdict::Compatible
    );
    assert_eq!(
        verdict("shannon_hartley", c * 1.01, "bit/s", &p),
        Verdict::Impossible
    );
    assert_eq!(
        verdict("shannon_hartley", 20.0, "Mbit/s", &p),
        Verdict::Impossible
    );
    // SNR of 1 gives exactly one bit per hertz.
    let one = [("B", 1000.0, "Hz"), ("snr", 1.0, "dimensionless")];
    assert_ne!(
        verdict("shannon_hartley", 1000.0, "bit/s", &one),
        Verdict::Impossible
    );
    assert_eq!(
        verdict("shannon_hartley", 1010.0, "bit/s", &one),
        Verdict::Impossible
    );
}
#[test]
fn shannon_hartley_missing_parameter_is_insufficient_data() {
    let v = verdict("shannon_hartley", 1.0e9, "bit/s", &[("B", 1.0, "MHz")]);
    assert!(matches!(v, Verdict::InsufficientData { .. }), "{v:?}");
}
proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var_os("CI").is_some() { 1024 } else { 256 },
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(42),
        ..ProptestConfig::default()
    })]
    /// A heat per erased bit at or above kB T ln 2 is never IMPOSSIBLE.
    #[test]
    fn landauer_at_or_above_the_bound_is_never_impossible(t in 1.0f64..1000.0, factor in 1.0f64..1.0e8) {
        let bound = KB * t * std::f64::consts::LN_2;
        prop_assert_ne!(verdict("landauer", bound * factor, "J", &[("T", t, "K")]), Verdict::Impossible);
    }
    /// A flux at or below sigma T^4 is never IMPOSSIBLE.
    #[test]
    fn stefan_boltzmann_at_or_below_the_bound_is_never_impossible(t in 10.0f64..10000.0, frac in 0.0f64..=1.0) {
        let bound = SIGMA_SB * t.powi(4);
        prop_assert_ne!(verdict("stefan_boltzmann", bound * frac, "W/m^2", &[("T", t, "K")]), Verdict::Impossible);
    }
    /// A rate at or below B log2(1 + S/N) is never IMPOSSIBLE.
    #[test]
    fn shannon_hartley_at_or_below_the_bound_is_never_impossible(
        b in 1.0e3f64..1.0e9, snr in 0.0f64..1.0e5, frac in 0.0f64..=1.0,
    ) {
        let bound = shannon(b, snr);
        let v = verdict("shannon_hartley", bound * frac, "bit/s",
            &[("B", b, "Hz"), ("snr", snr, "dimensionless")]);
        prop_assert_ne!(v, Verdict::Impossible);
    }
}
