//! Carnot limit through the decision engine (spec limit-carnot, AC 2 to 8).

mod common;

use common::scenario;
use limina_core::{
    decide, AbsoluteTemperature, Carnot, Claim, ConstraintsMap, Decision, DecisionConfig,
    DecisionInput, Dimension, EngineError, Measure, MissingStatus, ParameterId, Quantity, Verdict,
};
use proptest::prelude::*;

fn kelvin(k: f64) -> Measure {
    Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(k).expect("valid temperature"))
}

fn input(claim: f64, th: Option<f64>, tc: Option<f64>) -> DecisionInput {
    let value =
        Measure::Plain(Quantity::from_si(claim, Dimension::DIMENSIONLESS).expect("finite claim"));
    let mut known_parameters = std::collections::BTreeMap::new();
    if let Some(t) = th {
        known_parameters.insert(ParameterId::new("Th"), kelvin(t));
    }
    if let Some(t) = tc {
        known_parameters.insert(ParameterId::new("Tc"), kelvin(t));
    }
    DecisionInput {
        claim: Claim {
            value,
            uncertainty: None,
        },
        config: DecisionConfig::default(),
        constraints: ConstraintsMap::new(),
        known_parameters,
    }
}

fn run(claim: f64, th: f64, tc: f64) -> Result<Decision, EngineError> {
    decide(&Carnot::new(), &input(claim, Some(th), Some(tc)))
}

fn verdict(claim: f64, th: f64, tc: f64) -> Result<Verdict, EngineError> {
    run(claim, th, tc).map(|d| d.verdict)
}

/// Bound `b_max` reported by the engine for valid temperatures.
fn engine_bound(th: f64, tc: f64) -> f64 {
    let d = run(0.0, th, tc).expect("valid temperatures");
    let e = d.enclosures.expect("no missing parameter");
    assert_eq!(e.permissive.bounds.b_min, e.permissive.bounds.b_max);
    e.permissive.bounds.b_max
}

fn missing_ids(result: Result<Decision, EngineError>) -> Vec<(String, MissingStatus)> {
    match result.map(|d| d.verdict) {
        Ok(Verdict::InsufficientData { missing }) => {
            let mut ids: Vec<_> = missing
                .iter()
                .map(|m| (m.id.as_str().to_string(), m.status))
                .collect();
            ids.sort_by(|a, b| a.0.cmp(&b.0));
            ids
        }
        other => panic!("expected INSUFFICIENT_DATA, got {other:?}"),
    }
}

#[test]
fn ac2_030_with_600_300_is_compatible() {
    assert_eq!(verdict(0.30, 600.0, 300.0), Ok(Verdict::Compatible));
}

#[test]
fn ac3_060_with_600_300_is_impossible() {
    assert_eq!(verdict(0.60, 600.0, 300.0), Ok(Verdict::Impossible));
}

#[test]
fn claim_exactly_at_the_bound_is_compatible() {
    assert_eq!(verdict(0.5, 600.0, 300.0), Ok(Verdict::Compatible));
    assert_eq!(verdict(0.5, 500.0, 250.0), Ok(Verdict::Compatible));
}

#[test]
fn ac4_missing_temperatures_give_insufficient_data_listing_them() {
    for claim in [0.0, 0.30, 0.60, 5.0] {
        let both = decide(&Carnot::new(), &input(claim, None, None));
        let th_only = decide(&Carnot::new(), &input(claim, Some(600.0), None));
        let tc_only = decide(&Carnot::new(), &input(claim, None, Some(300.0)));
        assert_eq!(
            missing_ids(both),
            vec![
                ("Tc".to_string(), MissingStatus::NoDomain),
                ("Th".to_string(), MissingStatus::NoDomain)
            ],
            "claim {claim}"
        );
        assert_eq!(
            missing_ids(th_only),
            vec![("Tc".to_string(), MissingStatus::NoDomain)],
            "claim {claim}"
        );
        assert_eq!(
            missing_ids(tc_only),
            vec![("Th".to_string(), MissingStatus::NoDomain)],
            "claim {claim}"
        );
    }
}

#[test]
fn ac5_th_not_above_tc_is_the_typed_error() {
    let expected = Err(EngineError::InvalidTemperatureOrder {
        limit_id: "carnot".to_string(),
    });
    for (th, tc) in [(300.0, 300.0), (300.0, 600.0)] {
        for claim in [0.0, 0.30, 0.60] {
            assert_eq!(verdict(claim, th, tc), expected, "Th={th} Tc={tc}");
        }
    }
}

#[test]
fn zero_tc_is_the_typed_error() {
    assert_eq!(
        verdict(0.30, 300.0, 0.0),
        Err(EngineError::InvalidTemperatureOrder {
            limit_id: "carnot".to_string()
        })
    );
}

#[test]
fn engine_bound_matches_golden_values() {
    assert_eq!(engine_bound(600.0, 300.0), 0.5);
    assert_eq!(engine_bound(500.0, 250.0), 0.5);
}

/// `10^e` for an exponent drawn in `[lo, hi]`, so magnitudes are spread
/// over many decades instead of clustering at the top of a linear range.
fn decade(lo: f64, hi: f64) -> impl Strategy<Value = f64> {
    (lo..=hi).prop_map(|e| 10f64.powf(e))
}

/// Valid `(Th, Tc)` with `Tc` in `[1e-3, 1e4]` K and `Th = Tc * (1 + r)`,
/// `r` in `[1e-6, 1e6]`, so `Tc/Th >= ~1e-6` and `Th > Tc > 0` hold.
fn valid_pair() -> impl Strategy<Value = (f64, f64)> {
    (decade(-3.0, 4.0), decade(-6.0, 6.0)).prop_map(|(tc, r)| (tc * (1.0 + r), tc))
}

/// Independent oracle: `1 - Tc/Th` written in the test.
fn oracle(th: f64, tc: f64) -> f64 {
    1.0 - tc / th
}

proptest! {
    #![proptest_config(scenario::config())]

    /// AC 6. For valid `Tc > 0` the bound is strictly below 1 and equals the
    /// oracle. Holds in f64 on the generated range (`Tc/Th >= ~1e-6`).
    #[test]
    fn bound_is_strictly_below_one((th, tc) in valid_pair()) {
        let b = engine_bound(th, tc);
        prop_assert!(b < 1.0, "Th={:?} Tc={:?} bound={:?}", th, tc, b);
        prop_assert!(b > 0.0, "Th={:?} Tc={:?} bound={:?}", th, tc, b);
        prop_assert_eq!(b, oracle(th, tc), "Th={:?} Tc={:?}", th, tc);
    }

    /// AC 7. Increasing in `Th` at fixed `Tc`.
    #[test]
    fn bound_increases_with_th(
        (th, tc) in valid_pair(),
        factor in 1.001f64..=1000.0,
    ) {
        let th2 = th * factor;
        let (b1, b2) = (engine_bound(th, tc), engine_bound(th2, tc));
        prop_assert!(b1 < b2, "Tc={:?} Th={:?}->{:?} bounds {:?}->{:?}", tc, th, th2, b1, b2);
        prop_assert!(oracle(th, tc) < oracle(th2, tc));
    }

    /// AC 7. Decreasing in `Tc` at fixed `Th`.
    #[test]
    fn bound_decreases_with_tc(
        (th, tc) in valid_pair(),
        t in 0.001f64..=0.999,
    ) {
        // tc2 lies strictly between tc and th, so (th, tc2) is valid.
        let tc2 = tc + t * (th - tc);
        prop_assume!(tc2 > tc * 1.001 && tc2 < th);
        let (b1, b2) = (engine_bound(th, tc), engine_bound(th, tc2));
        prop_assert!(b1 > b2, "Th={:?} Tc={:?}->{:?} bounds {:?}->{:?}", th, tc, tc2, b1, b2);
        prop_assert!(oracle(th, tc) > oracle(th, tc2));
    }

    /// AC 8. A claim at or below the bound is never IMPOSSIBLE. The claim
    /// is built from the oracle bound, with the endpoint and its lower
    /// neighbour favoured.
    #[test]
    fn claim_at_or_below_bound_is_never_impossible(
        (th, tc) in valid_pair(),
        t in 0.0f64..=1.0,
        pick in 0u8..4,
    ) {
        let bound = oracle(th, tc);
        let claim = match pick {
            0 => bound,
            1 => bound.next_down(),
            _ => t * bound,
        };
        prop_assert!(claim <= bound, "generator broke its contract: {:?}", claim);
        let got = verdict(claim, th, tc);
        prop_assert!(
            got != Ok(Verdict::Impossible),
            "claim={:?} Th={:?} Tc={:?} gave {:?}", claim, th, tc, got
        );
    }
}
