//! P-04: equivalent units (W / kW / MW, J / kWh) give the same verdict and
//! margin.
//!
//! Integer variant: values are integer multiples, so every representation
//! has exactly the same SI value; the whole `Decision` must be equal.
//! Real-valued variant: representations may differ by rounding; the claim
//! is kept farther than the guard from every bound, and the verdicts must be
//! equal and the margins equal within D-06.

mod common;

use std::collections::BTreeMap;

use common::scenario::{config, guard_mode, GuardMode};
use common::TestLimit;
use limina_core::{
    decide, Claim, ConstraintsMap, Decision, DecisionConfig, DecisionInput, Dimension, Direction,
    Measure, MeasureKind, Quantity,
};
use proptest::prelude::*;

const POWER: Dimension = Dimension::new([2, 1, -3, 0, 0, 0, 0]);
const ENERGY: Dimension = Dimension::new([2, 1, -2, 0, 0, 0, 0]);

/// D-06 tolerance, written independently of the crate.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 + 1e-9 * a.abs().max(b.abs())
}

/// One assumption, one option per bound; bounds in SI base units.
fn limit(direction: Direction, dimension: Dimension, bounds_si: &[f64]) -> TestLimit {
    let options: Vec<String> = (0..bounds_si.len()).map(|i| i.to_string()).collect();
    let refs: Vec<&str> = options.iter().map(String::as_str).collect();
    let mut l = TestLimit::new(direction).with_assumption("a", &refs);
    for (o, b) in refs.iter().zip(bounds_si) {
        l = l.with_bound(&[o], *b, *b);
    }
    l.kind = MeasureKind::Plain(dimension);
    l
}

fn run(limit: &TestLimit, v: f64, u: f64, unit: &str, k: f64, guard: GuardMode) -> Decision {
    let (rel, abs) = guard.rel_abs();
    let q = |x: f64| Measure::Plain(Quantity::from_unit(x, unit).expect("valid quantity"));
    let input = DecisionInput {
        claim: Claim {
            value: q(v),
            uncertainty: Some(q(u)),
        },
        config: DecisionConfig::new(k, rel, abs, 100_000).expect("valid settings"),
        constraints: ConstraintsMap::new(),
        known_parameters: BTreeMap::new(),
    };
    decide(limit, &input).expect("valid input")
}

proptest! {
    #![proptest_config(config())]

    /// P-04, power: `n` MW, `1000 n` kW and `1e6 n` W give the same Decision.
    #[test]
    fn p04_power_units_give_the_same_decision(
        is_max in any::<bool>(),
        bounds_mw in proptest::collection::vec(0u32..=2000, 1..=4),
        n in 0u32..=2000,
        u in 0u32..=50,
        k in 1u32..=3,
        guard in guard_mode(),
    ) {
        let direction = if is_max { Direction::Max } else { Direction::Min };
        let bounds_si: Vec<f64> = bounds_mw.iter().map(|b| f64::from(*b) * 1e6).collect();
        let l = limit(direction, POWER, &bounds_si);
        let (n, u, k) = (f64::from(n), f64::from(u), f64::from(k));
        let mw = run(&l, n, u, "MW", k, guard);
        let kw = run(&l, n * 1e3, u * 1e3, "kW", k, guard);
        let w = run(&l, n * 1e6, u * 1e6, "W", k, guard);
        prop_assert_eq!(&mw, &kw);
        prop_assert_eq!(&mw, &w);
    }

    /// P-04, energy: `n` kWh and `3.6e6 n` J give the same Decision.
    #[test]
    fn p04_energy_units_give_the_same_decision(
        is_max in any::<bool>(),
        bounds_kwh in proptest::collection::vec(0u32..=2000, 1..=4),
        n in 0u32..=2000,
        u in 0u32..=50,
        k in 1u32..=3,
        guard in guard_mode(),
    ) {
        let direction = if is_max { Direction::Max } else { Direction::Min };
        let bounds_si: Vec<f64> = bounds_kwh.iter().map(|b| f64::from(*b) * 3.6e6).collect();
        let l = limit(direction, ENERGY, &bounds_si);
        let (n, u, k) = (f64::from(n), f64::from(u), f64::from(k));
        let kwh = run(&l, n, u, "kWh", k, guard);
        let j = run(&l, n * 3.6e6, u * 3.6e6, "J", k, guard);
        prop_assert_eq!(kwh, j);
    }

    /// P-04, real values: `x` MW and `1000 x` kW may differ by rounding in SI.
    /// Away from every bound and guard edge, verdict and margins agree.
    #[test]
    fn p04_real_valued_power_agrees_away_from_the_bounds(
        is_max in any::<bool>(),
        bounds_mw in proptest::collection::vec(0u32..=2000, 1..=4),
        x in 0.0..=2000.0f64,
    ) {
        let direction = if is_max { Direction::Max } else { Direction::Min };
        let bounds_si: Vec<f64> = bounds_mw.iter().map(|b| f64::from(*b) * 1e6).collect();
        let v_si = x * 1e6;
        // Default guard is about 1e-9 * b; stay much farther than that.
        prop_assume!(bounds_si.iter().all(|b| (v_si - b).abs() > 1e-6 * b.max(1.0) + 1.0));
        let l = limit(direction, POWER, &bounds_si);
        let mw = run(&l, x, 0.0, "MW", 1.0, GuardMode::Default);
        let kw = run(&l, x * 1e3, 0.0, "kW", 1.0, GuardMode::Default);
        prop_assert_eq!(&mw.verdict, &kw.verdict);
        let (a, b) = (mw.enclosures.expect("enclosures"), kw.enclosures.expect("enclosures"));
        prop_assert_eq!(a.permissive.bounds, b.permissive.bounds);
        match (a.permissive.margin, b.permissive.margin) {
            (Some(ma), Some(mb)) => {
                prop_assert!(close(ma.low, mb.low) && close(ma.high, mb.high), "{ma:?} vs {mb:?}");
            }
            (ma, mb) => prop_assert_eq!(ma, mb),
        }
    }
}
