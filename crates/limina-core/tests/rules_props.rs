//! Property tests of the verdict rules without missing parameters:
//! P-02, P-03 (a), E-01, E-02, E-04.
//!
//! Every property compares the engine with the independent oracle of
//! `common::scenario`, on scenarios generated with a fixed seed. A failure
//! prints the shrunk `Scenario` in readable form.
//!
//! P-07 to P-09 are out of scope here (D-05): they belong to limit-specific
//! specs and to the dataset.

mod common;

use common::scenario::{config, g, observed, oracle, predicates, scenario, Expected, Scenario};
use limina_core::{Direction, Verdict};
use proptest::prelude::*;

/// P-02 scenarios: a chosen configuration `c`, and a claim at or below
/// `B(c)` (Max-bound) or at or above it (Min-bound), or within the guard of
/// `B(c)`.
fn p02_scenario() -> impl Strategy<Value = Scenario> {
    (
        scenario(),
        0usize..27,
        0u32..=50,
        0.0..=1.0f64,
        any::<bool>(),
    )
        .prop_map(|(s, pick, t, frac, within_guard)| {
            let values = s.bound_values();
            let b = values[pick % values.len()];
            let (rel, abs) = s.guard.rel_abs();
            let gb = g(b, rel, abs);
            let (v, u) = match (s.direction, within_guard) {
                (Direction::Max, false) => (b - f64::from(t), s.u),
                (Direction::Max, true) => (f64::min(b + frac * gb, b + gb), 0.0),
                (Direction::Min, false) => (b + f64::from(t), s.u),
                (Direction::Min, true) => (f64::max(b - frac * gb, b - gb), 0.0),
            };
            s.with_claim(v, u)
        })
}

/// E-01 scenarios: no guard and `u = 0`.
fn e01_scenario() -> impl Strategy<Value = Scenario> {
    scenario().prop_map(|s| Scenario {
        u: 0.0,
        guard: common::scenario::GuardMode::Zero,
        ..s
    })
}

/// The original rules of the requirements (u = 0, no guard), written out.
fn original_rules(s: &Scenario) -> Expected {
    let v = s.v;
    let values = s.bound_values();
    let b_min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let b_max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let keep = |b: f64| match s.direction {
        Direction::Max => b >= v,
        Direction::Min => b <= v,
    };
    let c_star = s
        .bounds
        .iter()
        .filter(|(_, &b)| keep(b))
        .map(|(k, _)| k.clone())
        .collect();
    match s.direction {
        Direction::Max if v > b_max => Expected::Impossible,
        Direction::Max if v <= b_min => Expected::Compatible,
        Direction::Min if v < b_min => Expected::Impossible,
        Direction::Min if v >= b_max => Expected::Compatible,
        _ => Expected::Conditional(c_star),
    }
}

proptest! {
    #![proptest_config(config())]

    /// P-02: never IMPOSSIBLE for a value at or below `B(c)` (Max-bound;
    /// at or above for Min-bound), nor within the guard of any `B(c)`.
    #[test]
    fn p02_never_impossible_at_or_within_the_guard_of_a_bound(s in p02_scenario()) {
        let d = s.decide().expect("valid scenario");
        prop_assert_ne!(d.verdict, Verdict::Impossible);
    }

    /// P-03 (a): exactly one of the three rules applies, with the guarded
    /// boundaries, and the engine returns that verdict and that `C*`.
    #[test]
    fn p03a_exactly_one_rule_applies_and_the_engine_agrees(s in scenario()) {
        let p = predicates(&s);
        let conditional = !p.impossible && !p.compatible;
        let applied = [p.impossible, p.compatible, conditional]
            .iter()
            .filter(|x| **x)
            .count();
        prop_assert!(!(p.impossible && p.compatible), "IMPOSSIBLE and COMPATIBLE both hold");
        prop_assert_eq!(applied, 1);
        let d = s.decide().expect("valid scenario");
        prop_assert_eq!(observed(&d.verdict), oracle(&s));
    }

    /// E-01: with no guard and `u = 0`, the engine equals the original rules.
    #[test]
    fn e01_no_guard_and_no_uncertainty_give_the_original_rules(s in e01_scenario()) {
        let d = s.decide().expect("valid scenario");
        prop_assert_eq!(observed(&d.verdict), original_rules(&s));
    }

    /// E-02: increasing `u` never turns a non-IMPOSSIBLE verdict into
    /// IMPOSSIBLE, nor a non-COMPATIBLE verdict into COMPATIBLE.
    #[test]
    fn e02_more_uncertainty_never_creates_impossible_or_compatible(
        s in scenario(),
        du in 0u32..=50,
    ) {
        let wider = s.with_claim(s.v, s.u + f64::from(du));
        let narrow = s.decide().expect("valid scenario").verdict;
        let wide = wider.decide().expect("valid scenario").verdict;
        if narrow != Verdict::Impossible {
            prop_assert_ne!(&wide, &Verdict::Impossible, "wider claim: {:?}", wider);
        }
        if narrow != Verdict::Compatible {
            prop_assert_ne!(&wide, &Verdict::Compatible, "wider claim: {:?}", wider);
        }
    }

    /// E-04: `C*` is non-empty whenever the verdict is CONDITIONAL.
    #[test]
    fn e04_conditional_always_lists_a_configuration(s in scenario()) {
        let d = s.decide().expect("valid scenario");
        if let Verdict::Conditional { satisfying } = &d.verdict {
            prop_assert!(!satisfying.is_empty());
        }
    }
}
