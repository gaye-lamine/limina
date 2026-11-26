//! The laws added after the 26/11 review: escape speed (min), power
//! conservation, Shockley-Queisser, Rayleigh criterion, mass-energy
//! equivalence. Expected values are computed here from the textbook
//! formulas with the constants written out, not by the DSL.
use limina_core::{
    decide, AssumptionId, Claim, DecisionConfig, DecisionInput, Measure, OptionId, ParameterId,
    Quantity, Verdict,
};
use limina_dsl::Registry;
use proptest::prelude::*;
use std::collections::BTreeMap;

const C: f64 = 299_792_458.0;
/// IAU 2015 nominal Earth GM and equatorial radius.
const GM_EARTH: f64 = 3.986_004e14;
const R_EARTH: f64 = 6.378_1e6;

type Params<'a> = &'a [(&'a str, f64, &'a str)];

fn input(claim: f64, unit: &str, params: Params) -> DecisionInput {
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

fn verdict(
    id: &str,
    claim: f64,
    unit: &str,
    params: Params,
    option: Option<(&str, &str)>,
) -> Verdict {
    let registry = Registry::builtin().expect("builtin");
    let law = registry.get(id).expect("law");
    let mut i = input(claim, unit, params);
    if let Some((a, o)) = option {
        i.constraints.allow(
            AssumptionId::new(a),
            [OptionId::new(o)].into_iter().collect(),
        );
    }
    decide(law, &i).expect("decides").verdict
}

fn escape(gm: f64, r: f64) -> f64 {
    (2.0 * gm / r).sqrt()
}

const EARTH: [(&str, f64, &str); 2] = [("GM", GM_EARTH, "m^3/s^2"), ("r", R_EARTH, "m")];
const BALLISTIC: Option<(&str, &str)> = Some(("propulsion", "ballistic"));
const INPUT_ONLY: Option<(&str, &str)> = Some(("energy_sources", "input_only"));
const ONE_SUN: Option<(&str, &str)> = Some(("cell_type", "single_junction_one_sun"));
const TELESCOPE: [(&str, f64, &str); 2] = [("wavelength", 550.0, "nm"), ("D", 0.10, "m")];

#[test]
fn every_new_law_has_a_classification_and_a_real_reference() {
    let registry = Registry::builtin().expect("builtin");
    for id in [
        "escape_speed_min",
        "power_conservation",
        "shockley_queisser",
        "rayleigh_criterion",
        "mass_energy",
    ] {
        let def = registry
            .get(id)
            .map(|l| l.definition().clone())
            .unwrap_or_else(|| panic!("{id} is not built in"));
        assert!(def.classification.is_some(), "{id}: classification");
        assert!(def.has_real_reference(), "{id}: reference");
        assert!(def.validity_domain.is_some(), "{id}: validity domain");
        assert!(!def.limitations.is_empty(), "{id}: limitations");
    }
}

#[test]
fn known_textbook_values() {
    // Escape speed at the Earth's surface: 11.18 km/s.
    assert!((escape(GM_EARTH, R_EARTH) - 11_179.9).abs() < 0.1);
    // Rayleigh criterion, 550 nm through 0.10 m: 6.71e-6 rad.
    assert!((1.22_f64 * 550e-9 / 0.10 - 6.71e-6).abs() < 1e-9);
    // 1 kg of mass: 8.988e16 J.
    assert!((C * C - 8.987_551_787e16).abs() < 1e7);
}

#[test]
fn escape_speed_golden_and_negative() {
    let v = escape(GM_EARTH, R_EARTH);
    assert_eq!(
        verdict("escape_speed_min", 8.0, "km/s", &EARTH, BALLISTIC),
        Verdict::Impossible
    );
    assert_eq!(
        verdict("escape_speed_min", 12.0, "km/s", &EARTH, BALLISTIC),
        Verdict::Compatible
    );
    assert_eq!(
        verdict("escape_speed_min", v * 1.01, "m/s", &EARTH, BALLISTIC),
        Verdict::Compatible
    );
    // Propulsion not stated: 8 km/s is possible only with propulsion.
    assert!(matches!(
        verdict("escape_speed_min", 8.0, "km/s", &EARTH, None),
        Verdict::Conditional { .. }
    ));
    let missing = verdict(
        "escape_speed_min",
        8.0,
        "km/s",
        &[("r", R_EARTH, "m")],
        BALLISTIC,
    );
    assert!(
        matches!(missing, Verdict::InsufficientData { .. }),
        "{missing:?}"
    );
}

#[test]
fn power_conservation_golden_and_negative() {
    let p = [("P_in", 100.0, "W")];
    assert_eq!(
        verdict("power_conservation", 120.0, "W", &p, INPUT_ONLY),
        Verdict::Impossible
    );
    assert_eq!(
        verdict("power_conservation", 95.0, "W", &p, INPUT_ONLY),
        Verdict::Compatible
    );
    assert_eq!(
        verdict(
            "power_conservation",
            120.0,
            "W",
            &p,
            Some(("energy_sources", "other_sources"))
        ),
        Verdict::Compatible
    );
    assert!(matches!(
        verdict("power_conservation", 120.0, "W", &p, None),
        Verdict::Conditional { .. }
    ));
}

#[test]
fn shockley_queisser_golden_and_negative() {
    assert_eq!(
        verdict("shockley_queisser", 0.45, "dimensionless", &[], ONE_SUN),
        Verdict::Impossible
    );
    assert_eq!(
        verdict("shockley_queisser", 0.26, "dimensionless", &[], ONE_SUN),
        Verdict::Compatible
    );
    // Cell type not stated: 45 % is possible only for a tandem or concentrator cell.
    assert!(matches!(
        verdict("shockley_queisser", 0.45, "dimensionless", &[], None),
        Verdict::Conditional { .. }
    ));
    // Above the infinite-stack limit (86.8 %) in every configuration.
    assert_eq!(
        verdict("shockley_queisser", 0.9, "dimensionless", &[], None),
        Verdict::Impossible
    );
}

#[test]
fn rayleigh_criterion_golden_and_negative() {
    assert_eq!(
        verdict("rayleigh_criterion", 1e-5, "rad", &TELESCOPE, None),
        Verdict::Compatible
    );
    // Below the Sparrow value too: finer than any criterion allows.
    assert_eq!(
        verdict("rayleigh_criterion", 5e-6, "rad", &TELESCOPE, None),
        Verdict::Impossible
    );
    // Between Sparrow (5.21e-6) and Rayleigh (6.71e-6): depends on the criterion.
    assert!(matches!(
        verdict("rayleigh_criterion", 6e-6, "rad", &TELESCOPE, None),
        Verdict::Conditional { .. }
    ));
    let missing = verdict(
        "rayleigh_criterion",
        1e-5,
        "rad",
        &[("wavelength", 550.0, "nm")],
        None,
    );
    assert!(
        matches!(missing, Verdict::InsufficientData { .. }),
        "{missing:?}"
    );
}

#[test]
fn mass_energy_golden_and_negative() {
    let p = [("dm", 1.0, "kg")];
    assert_eq!(
        verdict("mass_energy", 2e17, "J", &p, None),
        Verdict::Impossible
    );
    assert_eq!(
        verdict("mass_energy", 5e16, "J", &p, None),
        Verdict::Compatible
    );
    assert_eq!(
        verdict("mass_energy", 9.0e19, "J", &[("dm", 1.0, "g")], None),
        Verdict::Impossible
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var("CI").is_ok() { 1024 } else { 256 },
        rng_seed: proptest::test_runner::RngSeed::Fixed(42),
        ..ProptestConfig::default()
    })]

    /// A claim at or inside the bound is never IMPOSSIBLE (R-03).
    #[test]
    fn a_claim_inside_the_bound_is_never_impossible(
        f in 0.0f64..=1.0,
        gm in 1e10f64..1e20,
        r in 1e3f64..1e9,
        p_in in 1e-3f64..1e9,
        wl in 100.0f64..2000.0,
        d in 1e-3f64..10.0,
        dm in 1e-6f64..1e3,
    ) {
        // Min laws: at or above the bound.
        let v = escape(gm, r) * (1.0 + f);
        let got = verdict("escape_speed_min", v, "m/s", &[("GM", gm, "m^3/s^2"), ("r", r, "m")], BALLISTIC);
        prop_assert_ne!(got, Verdict::Impossible, "escape speed {} m/s, GM {} m^3/s^2, r {} m", v, gm, r);
        let theta = 1.22 * wl * 1e-9 / d * (1.0 + f);
        let got = verdict("rayleigh_criterion", theta, "rad", &[("wavelength", wl, "nm"), ("D", d, "m")], None);
        prop_assert_ne!(got, Verdict::Impossible, "angle {} rad, wavelength {} nm, D {} m", theta, wl, d);
        // Max laws: at or below the bound.
        let got = verdict("power_conservation", p_in * f, "W", &[("P_in", p_in, "W")], INPUT_ONLY);
        prop_assert_ne!(got, Verdict::Impossible, "output {} W, input {} W", p_in * f, p_in);
        let got = verdict("shockley_queisser", 0.337 * f, "dimensionless", &[], ONE_SUN);
        prop_assert_ne!(got, Verdict::Impossible, "efficiency {}", 0.337 * f);
        let e = dm * C * C * f;
        let got = verdict("mass_energy", e, "J", &[("dm", dm, "kg")], None);
        prop_assert_ne!(got, Verdict::Impossible, "energy {} J, mass {} kg", e, dm);
    }
}
