//! COP limit through the decision engine (spec limit-cop, AC 2 to 6).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::scenario;
use limina_core::{
    approx_eq, decide, AbsoluteTemperature, AssumptionId, Claim, ConstraintsMap, Cop, Decision,
    DecisionConfig, DecisionInput, Dimension, EngineError, Measure, MissingStatus, OptionId,
    ParameterId, Quantity, Support, Verdict,
};
use proptest::prelude::*;

/// `mode` as given to the engine: one option, or unconstrained.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Heating,
    Cooling,
    Unconstrained,
}

fn kelvin(k: f64) -> Measure {
    Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(k).expect("valid temperature"))
}

fn input(claim: f64, mode: Mode, th: Option<f64>, tc: Option<f64>) -> DecisionInput {
    let value =
        Measure::Plain(Quantity::from_si(claim, Dimension::DIMENSIONLESS).expect("finite claim"));
    let mut known_parameters = BTreeMap::new();
    if let Some(t) = th {
        known_parameters.insert(ParameterId::new("Th"), kelvin(t));
    }
    if let Some(t) = tc {
        known_parameters.insert(ParameterId::new("Tc"), kelvin(t));
    }
    let mut constraints = ConstraintsMap::new();
    let option = match mode {
        Mode::Heating => Some("heating"),
        Mode::Cooling => Some("cooling"),
        Mode::Unconstrained => None,
    };
    if let Some(o) = option {
        constraints.allow(
            AssumptionId::new("mode"),
            BTreeSet::from([OptionId::new(o)]),
        );
    }
    DecisionInput {
        claim: Claim {
            value,
            uncertainty: None,
        },
        config: DecisionConfig::default(),
        constraints,
        known_parameters,
    }
}

fn run(claim: f64, mode: Mode, th: f64, tc: f64) -> Result<Decision, EngineError> {
    decide(&Cop::new(), &input(claim, mode, Some(th), Some(tc)))
}

fn verdict(claim: f64, mode: Mode, th: f64, tc: f64) -> Result<Verdict, EngineError> {
    run(claim, mode, th, tc).map(|d| d.verdict)
}

/// Bound reported by the engine for one mode and valid temperatures.
fn engine_bound(mode: Mode, th: f64, tc: f64) -> f64 {
    let d = run(0.0, mode, th, tc).expect("valid temperatures");
    let e = d.enclosures.expect("no missing parameter");
    assert_eq!(e.permissive.bounds.b_min, e.permissive.bounds.b_max);
    e.permissive.bounds.b_max
}

#[test]
fn claim_10_is_compatible() {
    assert_eq!(
        verdict(10.0, Mode::Unconstrained, 300.0, 280.0),
        Ok(Verdict::Compatible)
    );
}

#[test]
fn claim_16_is_impossible() {
    assert_eq!(
        verdict(16.0, Mode::Unconstrained, 300.0, 280.0),
        Ok(Verdict::Impossible)
    );
}

#[test]
fn claim_14_5_unconstrained_is_conditional_on_heating_only_certain() {
    match verdict(14.5, Mode::Unconstrained, 300.0, 280.0) {
        Ok(Verdict::Conditional { satisfying }) => {
            assert_eq!(satisfying.len(), 1, "satisfying set {satisfying:?}");
            let member = &satisfying[0];
            let options: Vec<(&str, &str)> = member
                .config
                .iter()
                .map(|(a, o)| (a.as_str(), o.as_str()))
                .collect();
            assert_eq!(options, [("mode", "heating")]);
            assert_eq!(member.support, Support::Certain);
        }
        other => panic!("expected CONDITIONAL, got {other:?}"),
    }
}

#[test]
fn claim_14_5_heating_is_compatible() {
    assert_eq!(
        verdict(14.5, Mode::Heating, 300.0, 280.0),
        Ok(Verdict::Compatible)
    );
}

#[test]
fn claim_14_5_cooling_is_impossible() {
    assert_eq!(
        verdict(14.5, Mode::Cooling, 300.0, 280.0),
        Ok(Verdict::Impossible)
    );
}

#[test]
fn engine_bounds_match_golden_values() {
    for (th, tc, heating, cooling) in [
        (300.0, 280.0, 15.0, 14.0),
        (320.0, 280.0, 8.0, 7.0),
        (300.0, 250.0, 6.0, 5.0),
    ] {
        assert_eq!(engine_bound(Mode::Heating, th, tc), heating);
        assert_eq!(engine_bound(Mode::Cooling, th, tc), cooling);
    }
}

#[test]
fn a_missing_temperature_gives_insufficient_data() {
    for mode in [Mode::Heating, Mode::Cooling, Mode::Unconstrained] {
        for (th, tc, missing) in [
            (None, None, vec!["Tc", "Th"]),
            (Some(300.0), None, vec!["Tc"]),
            (None, Some(280.0), vec!["Th"]),
        ] {
            match decide(&Cop::new(), &input(14.5, mode, th, tc)).map(|d| d.verdict) {
                Ok(Verdict::InsufficientData { missing: got }) => {
                    let mut ids: Vec<&str> = got.iter().map(|m| m.id.as_str()).collect();
                    ids.sort_unstable();
                    assert_eq!(ids, missing, "{mode:?}");
                    assert!(got.iter().all(|m| m.status == MissingStatus::NoDomain));
                }
                other => panic!("expected INSUFFICIENT_DATA for {mode:?}, got {other:?}"),
            }
        }
    }
}

#[test]
fn th_not_above_tc_is_the_typed_error() {
    let expected = Err(EngineError::InvalidTemperatureOrder {
        limit_id: "cop".to_string(),
    });
    for mode in [Mode::Heating, Mode::Cooling, Mode::Unconstrained] {
        for (th, tc) in [(280.0, 280.0), (280.0, 300.0), (300.0, 0.0)] {
            assert_eq!(
                verdict(14.5, mode, th, tc),
                expected,
                "{mode:?} Th={th} Tc={tc}"
            );
        }
    }
}

/// `10^e` for an exponent drawn in `[lo, hi]`.
fn decade(lo: f64, hi: f64) -> impl Strategy<Value = f64> {
    (lo..=hi).prop_map(|e| 10f64.powf(e))
}

/// Valid `(Th, Tc)` with `Tc` in `[1e-3, 1e4]` K and `Th = Tc * (1 + r)`,
/// `r` in `[1e-6, 1e6]`, so `Th > Tc > 0` holds.
fn valid_pair() -> impl Strategy<Value = (f64, f64)> {
    (decade(-3.0, 4.0), decade(-6.0, 6.0)).prop_map(|(tc, r)| (tc * (1.0 + r), tc))
}

/// Independent oracles, written in the test.
fn oracle_heating(th: f64, tc: f64) -> f64 {
    th / (th - tc)
}

fn oracle_cooling(th: f64, tc: f64) -> f64 {
    tc / (th - tc)
}

fn mode_choice() -> impl Strategy<Value = Mode> {
    prop_oneof![
        Just(Mode::Heating),
        Just(Mode::Cooling),
        Just(Mode::Unconstrained)
    ]
}

proptest! {
    #![proptest_config(scenario::config())]

    /// AC 4. Heating bound = cooling bound + 1, within the engine tolerance.
    #[test]
    fn heating_is_cooling_plus_one((th, tc) in valid_pair()) {
        let heating = engine_bound(Mode::Heating, th, tc);
        let cooling = engine_bound(Mode::Cooling, th, tc);
        prop_assert!(
            approx_eq(heating, cooling + 1.0),
            "Th={:?} Tc={:?} heating={:?} cooling={:?}", th, tc, heating, cooling
        );
        prop_assert_eq!(heating, oracle_heating(th, tc), "Th={:?} Tc={:?}", th, tc);
        prop_assert_eq!(cooling, oracle_cooling(th, tc), "Th={:?} Tc={:?}", th, tc);
    }

    /// AC 5. Both bounds decrease as `Th - Tc` grows at fixed `Tc`.
    #[test]
    fn bounds_decrease_as_the_gap_grows(
        (th, tc) in valid_pair(),
        factor in 1.001f64..=1000.0,
    ) {
        let th2 = tc + (th - tc) * factor;
        prop_assume!(th2 > th);
        for mode in [Mode::Heating, Mode::Cooling] {
            let (b1, b2) = (engine_bound(mode, th, tc), engine_bound(mode, th2, tc));
            prop_assert!(
                b1 > b2,
                "{:?} Tc={:?} Th={:?}->{:?} bounds {:?}->{:?}", mode, tc, th, th2, b1, b2
            );
        }
        prop_assert!(oracle_heating(th, tc) > oracle_heating(th2, tc));
        prop_assert!(oracle_cooling(th, tc) > oracle_cooling(th2, tc));
    }

    /// AC 6. A claim at or below the bound is never IMPOSSIBLE. With `mode`
    /// unconstrained the bound is the largest one, heating.
    #[test]
    fn claim_at_or_below_bound_is_never_impossible(
        (th, tc) in valid_pair(),
        mode in mode_choice(),
        t in 0.0f64..=1.0,
        pick in 0u8..4,
    ) {
        let bound = match mode {
            Mode::Cooling => oracle_cooling(th, tc),
            Mode::Heating | Mode::Unconstrained => oracle_heating(th, tc),
        };
        let claim = match pick {
            0 => bound,
            1 => bound.next_down(),
            _ => t * bound,
        };
        prop_assert!(claim <= bound, "generator broke its contract: {:?}", claim);
        let got = verdict(claim, mode, th, tc);
        prop_assert!(
            got != Ok(Verdict::Impossible),
            "claim={:?} mode={:?} Th={:?} Tc={:?} gave {:?}", claim, mode, th, tc, got
        );
    }
}
