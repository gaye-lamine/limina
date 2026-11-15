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

use std::fmt;

use common::scenario::{config, g, observed, oracle, predicates, scenario, Expected, Scenario};
use limina_core::{
    decide, Dimension, Direction, DomainRange, Interval, MeasureKind, MissingStatus, ParameterId,
    ParameterSpec, PlausibleDomain, Quantity, Support, Verdict,
};
use proptest::prelude::*;

/// A parameter of the generated limit: required or not, with or without a
/// plausible domain, supplied with a positive value or missing.
#[derive(Clone, Debug)]
struct Param {
    required: bool,
    has_domain: bool,
    supplied: Option<u32>,
}

/// A scenario plus 0 to 3 parameters (P-03 (b)).
#[derive(Clone)]
struct ParamScenario {
    base: Scenario,
    params: Vec<Param>,
}

impl fmt::Debug for ParamScenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} | params {:?}", self.base, self.params)
    }
}

impl ParamScenario {
    fn id(i: usize) -> ParameterId {
        ParameterId::new(format!("p{i}"))
    }

    /// Ids and statuses of the required parameters that are not supplied.
    fn expected_missing(&self) -> Vec<(ParameterId, MissingStatus)> {
        self.params
            .iter()
            .enumerate()
            .filter(|(_, p)| p.required && p.supplied.is_none())
            .map(|(i, p)| {
                let status = if p.has_domain {
                    MissingStatus::HasDomain
                } else {
                    MissingStatus::NoDomain
                };
                (Self::id(i), status)
            })
            .collect()
    }
}

fn param_scenario() -> impl Strategy<Value = ParamScenario> {
    let param = (any::<bool>(), any::<bool>(), proptest::option::of(1u32..=5)).prop_map(
        |(required, has_domain, supplied)| Param {
            required,
            has_domain,
            supplied,
        },
    );
    (scenario(), proptest::collection::vec(param, 0..=3))
        .prop_map(|(base, params)| ParamScenario { base, params })
}

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

    /// P-03 (b): with or without missing parameters, `decide` returns exactly
    /// one verdict with a consistent payload, and never panics.
    #[test]
    fn p03b_exactly_one_verdict_with_a_consistent_payload(ps in param_scenario()) {
        let dimless = MeasureKind::Plain(Dimension::DIMENSIONLESS);
        let mut limit = ps.base.limit();
        let mut input = ps.base.input();
        for (i, p) in ps.params.iter().enumerate() {
            let plausible = p.has_domain.then(|| PlausibleDomain {
                range: DomainRange::Interval(Interval::new(1.0, 5.0).expect("valid")),
                source: "test".to_string(),
            });
            limit = limit.with_parameter(ParameterSpec {
                id: ParamScenario::id(i),
                kind: dimless,
                required: p.required,
                plausible,
            });
            if let Some(x) = p.supplied {
                let q = Quantity::from_si(f64::from(x), Dimension::DIMENSIONLESS).expect("valid");
                input
                    .known_parameters
                    .insert(ParamScenario::id(i), limina_core::Measure::Plain(q));
            }
        }
        let d = decide(&limit, &input).expect("valid input never errors");
        let expected_missing = ps.expected_missing();

        for e in [d.enclosures.map(|e| e.permissive), d.enclosures.map(|e| e.restrictive)]
            .into_iter()
            .flatten()
        {
            prop_assert!(e.bounds.b_min <= e.bounds.b_max);
            if let Some(m) = e.margin {
                prop_assert!(m.low <= m.high, "margin {m:?}");
            }
        }
        match &d.verdict {
            Verdict::InsufficientData { missing } => {
                prop_assert!(!missing.is_empty());
                let got: Vec<(ParameterId, MissingStatus)> =
                    missing.iter().map(|m| (m.id.clone(), m.status)).collect();
                prop_assert_eq!(got, expected_missing);
            }
            Verdict::Conditional { satisfying } => {
                prop_assert!(!satisfying.is_empty());
                prop_assert!(d.enclosures.is_some());
                if expected_missing.is_empty() {
                    prop_assert!(satisfying.iter().all(|s| s.support == Support::Certain));
                }
            }
            Verdict::Impossible | Verdict::Compatible => {
                prop_assert!(d.enclosures.is_some());
            }
        }
        if d.enclosures.is_none() {
            let is_insufficient = matches!(d.verdict, Verdict::InsufficientData { .. });
            prop_assert!(is_insufficient);
        }
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
