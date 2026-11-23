//! Property tests with missing parameters and plausible domains (T30):
//! P-02 (permissive variant), P-03 (b), E-02, E-04, covering R-05 and R-09.
//!
//! The limit is `B(c, p) = base(c) * p1 * p2` (arbitrary test function, no
//! physics). Every parameter is positive, so the enclosure ends are plain
//! products and the test oracle computes them itself, without calling any
//! engine helper. The oracle also owns its guard formula and its combination
//! rule: IMPOSSIBLE only when both enclosures give IMPOSSIBLE, equal verdicts
//! returned as they are, different verdicts give INSUFFICIENT_DATA, and
//! `Certain` decided by the guarded comparison on the restrictive end (D-19).
//!
//! P-07 to P-09 are out of scope here (D-05).

mod common;

use std::fmt;

use common::scenario::{config, g, scenario, Scenario};
use limina_core::{
    decide, Decision, Dimension, Direction, DomainRange, Interval, Measure, MeasureKind,
    MissingStatus, ParameterId, ParameterSpec, PlausibleDomain, Quantity, Support, Verdict,
};
use proptest::prelude::*;

/// A parameter of the generated limit, always required.
#[derive(Clone, Debug)]
struct MParam {
    lo: u32,
    hi: u32,
    /// Declared plausible domain `[lo, hi]`; when false the limit declares none.
    has_domain: bool,
    /// Value supplied by the claim, if any; otherwise the parameter is missing.
    supplied: Option<u32>,
}

/// A scenario plus 0 to 2 parameters.
#[derive(Clone)]
struct MScenario {
    base: Scenario,
    params: Vec<MParam>,
}

impl fmt::Debug for MScenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params: Vec<String> = self
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| match (p.supplied, p.has_domain) {
                (Some(s), _) => format!("p{i}={s}"),
                (None, true) => format!("p{i} missing in [{},{}]", p.lo, p.hi),
                (None, false) => format!("p{i} missing, no domain"),
            })
            .collect();
        write!(f, "{:?} | {}", self.base, params.join("; "))
    }
}

impl MScenario {
    fn id(i: usize) -> ParameterId {
        ParameterId::new(format!("p{i}"))
    }

    fn limit(&self) -> common::TestLimit {
        let mut limit = self.base.limit();
        for (i, p) in self.params.iter().enumerate() {
            let plausible = p.has_domain.then(|| PlausibleDomain {
                range: DomainRange::Interval(
                    Interval::new(f64::from(p.lo), f64::from(p.hi)).expect("valid domain"),
                ),
                source: "test".to_string(),
            });
            limit = limit.with_parameter(ParameterSpec {
                id: Self::id(i),
                kind: MeasureKind::Plain(Dimension::DIMENSIONLESS),
                required: true,
                plausible,
            });
        }
        limit
    }

    fn decide(&self) -> Decision {
        let mut input = self.base.input();
        for (i, p) in self.params.iter().enumerate() {
            if let Some(s) = p.supplied {
                let q = Quantity::from_si(f64::from(s), Dimension::DIMENSIONLESS).expect("finite");
                input
                    .known_parameters
                    .insert(Self::id(i), Measure::Plain(q));
            }
        }
        decide(&self.limit(), &input).expect("valid input never errors")
    }

    fn with_claim(&self, v: f64, u: f64) -> MScenario {
        MScenario {
            base: self.base.with_claim(v, u),
            params: self.params.clone(),
        }
    }

    /// Required parameters that are not supplied, in declaration order.
    fn missing(&self) -> Vec<(ParameterId, MissingStatus)> {
        self.params
            .iter()
            .enumerate()
            .filter(|(_, p)| p.supplied.is_none())
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

    fn any_no_domain(&self) -> bool {
        self.missing()
            .iter()
            .any(|(_, s)| *s == MissingStatus::NoDomain)
    }

    /// Per configuration (key order): the smallest and the largest value of
    /// `B(c, p)` over the plausible ranges. Valid when no missing parameter
    /// lacks a domain.
    fn extremes(&self) -> Vec<(f64, f64)> {
        self.base
            .bound_values()
            .into_iter()
            .map(|b| {
                let (mut low, mut high) = (b, b);
                for p in &self.params {
                    let (a, z) = match p.supplied {
                        Some(s) => (f64::from(s), f64::from(s)),
                        None => (f64::from(p.lo), f64::from(p.hi)),
                    };
                    low *= a;
                    high *= z;
                }
                (low, high)
            })
            .collect()
    }

    /// `(permissive, restrictive)` value of `B` per configuration.
    fn ends(&self) -> Vec<(f64, f64)> {
        self.extremes()
            .into_iter()
            .map(|(low, high)| match self.base.direction {
                Direction::Max => (high, low),
                Direction::Min => (low, high),
            })
            .collect()
    }
}

/// Rule outcome under one enclosure.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Rule {
    Impossible,
    Compatible,
    Conditional,
}

/// Rules of R-03 / R-04 with the guard, over the values of one enclosure.
fn rule(s: &Scenario, vals: &[f64]) -> Rule {
    let (lo, hi) = s.claim_interval();
    let (rel, abs) = s.guard.rel_abs();
    let min = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let max = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    match s.direction {
        Direction::Max if vals.iter().all(|&b| lo > b + g(b, rel, abs)) => Rule::Impossible,
        Direction::Max if hi <= min => Rule::Compatible,
        Direction::Min if vals.iter().all(|&b| hi < b - g(b, rel, abs)) => Rule::Impossible,
        Direction::Min if lo >= max => Rule::Compatible,
        _ => Rule::Conditional,
    }
}

/// Guarded membership test of `C*` for one value of `B`.
fn satisfies(s: &Scenario, b: f64) -> bool {
    let (lo, hi) = s.claim_interval();
    let (rel, abs) = s.guard.rel_abs();
    match s.direction {
        Direction::Max => b + g(b, rel, abs) >= lo,
        Direction::Min => b - g(b, rel, abs) <= hi,
    }
}

/// Expected outcome, from the raw numbers.
#[derive(Debug, PartialEq)]
enum Expect {
    Impossible,
    Compatible,
    /// `C*` keys with their `Certain` flag.
    Conditional(Vec<(Vec<String>, bool)>),
    Insufficient,
}

fn expect(m: &MScenario) -> Expect {
    let ends = m.ends();
    let permissive: Vec<f64> = ends.iter().map(|e| e.0).collect();
    let restrictive: Vec<f64> = ends.iter().map(|e| e.1).collect();
    let (rp, rr) = (rule(&m.base, &permissive), rule(&m.base, &restrictive));
    if rp != rr {
        return Expect::Insufficient;
    }
    match rp {
        Rule::Impossible => Expect::Impossible,
        Rule::Compatible => Expect::Compatible,
        Rule::Conditional => Expect::Conditional(
            m.base
                .bounds
                .keys()
                .zip(ends.iter())
                .filter(|(_, e)| satisfies(&m.base, e.0))
                .map(|(key, e)| (key.clone(), satisfies(&m.base, e.1)))
                .collect(),
        ),
    }
}

fn observe(d: &Decision, missing_ids: &[ParameterId]) -> Result<Expect, String> {
    Ok(match &d.verdict {
        Verdict::Impossible => Expect::Impossible,
        Verdict::Compatible => Expect::Compatible,
        Verdict::InsufficientData { .. } => Expect::Insufficient,
        Verdict::Conditional { satisfying } => {
            let mut out = Vec::new();
            for s in satisfying {
                let key = s
                    .config
                    .iter()
                    .map(|(_, o)| o.as_str().to_string())
                    .collect();
                let certain = match &s.support {
                    Support::Certain => true,
                    Support::Depends(ids) if ids == missing_ids => false,
                    Support::Depends(ids) => {
                        return Err(format!("Depends lists {ids:?}, missing is {missing_ids:?}"))
                    }
                };
                out.push((key, certain));
            }
            Expect::Conditional(out)
        }
    })
}

fn edge(direction: Direction, b: f64, rel: f64, abs: f64, step: i8) -> f64 {
    let e = match direction {
        Direction::Max => b + g(b, rel, abs),
        Direction::Min => b - g(b, rel, abs),
    };
    match step {
        s if s < 0 => e.next_down(),
        0 => e,
        _ => e.next_up(),
    }
}

fn param() -> impl Strategy<Value = MParam> {
    (
        1u32..=5,
        0u32..=3,
        prop_oneof![3 => Just(true), 1 => Just(false)],
        proptest::option::weighted(0.3, 1u32..=5),
    )
        .prop_map(|(lo, width, has_domain, supplied)| MParam {
            lo,
            hi: lo + width,
            has_domain,
            supplied,
        })
}

/// Scenarios with 0 to 2 parameters. The claim is either the generated one,
/// or placed on the permissive or restrictive end of a chosen configuration,
/// or on the guard edge of one of them (the D-19 boundary).
fn m_scenario() -> impl Strategy<Value = MScenario> {
    (
        scenario(),
        proptest::collection::vec(param(), 0..=2),
        (0usize..27, -2i32..=2, 0u8..5, -1i8..=1),
    )
        .prop_map(|(base, params, (pick, off, mode, step))| {
            let m = MScenario { base, params };
            if m.any_no_domain() {
                return m;
            }
            let ends = m.ends();
            let (perm, restr) = ends[pick % ends.len()];
            let (rel, abs) = m.base.guard.rel_abs();
            let dir = m.base.direction;
            let (v, u) = match mode {
                0 => (m.base.v, m.base.u),
                1 => (perm + f64::from(off), m.base.u),
                2 => (restr + f64::from(off), m.base.u),
                3 => (edge(dir, restr, rel, abs, step), 0.0),
                _ => (edge(dir, perm, rel, abs, step), 0.0),
            };
            m.with_claim(v, u)
        })
}

/// P-02 scenarios: all parameters missing with a domain, plus a value
/// `p*` inside each plausible range.
fn p02_scenario() -> impl Strategy<Value = (MScenario, Vec<f64>)> {
    let missing = (1u32..=5, 0u32..=3).prop_map(|(lo, width)| MParam {
        lo,
        hi: lo + width,
        has_domain: true,
        supplied: None,
    });
    (
        scenario(),
        proptest::collection::vec(missing, 1..=2),
        proptest::collection::vec(0.0..=1.0f64, 2),
        (0usize..27, -2i32..=2, 0u8..4, 0u32..=50, 0.0..=1.0f64),
    )
        .prop_map(|(base, params, fractions, (pick, off, mode, t, frac))| {
            let pstar: Vec<f64> = params
                .iter()
                .zip(fractions.iter())
                .map(|(p, t)| {
                    let (lo, hi) = (f64::from(p.lo), f64::from(p.hi));
                    f64::min(lo + t * (hi - lo), hi)
                })
                .collect();
            let m = MScenario { base, params };
            let values = pstar_values(&m, &pstar);
            let b = values[pick % values.len()];
            let (rel, abs) = m.base.guard.rel_abs();
            let gb = g(b, rel, abs);
            let max = m.base.direction == Direction::Max;
            let (v, u) = match mode {
                0 => (m.base.v, m.base.u),
                1 => (b + f64::from(off), m.base.u),
                2 if max => (b - f64::from(t), m.base.u),
                2 => (b + f64::from(t), m.base.u),
                _ if max => (f64::min(b + frac * gb, b + gb), 0.0),
                _ => (f64::max(b - frac * gb, b - gb), 0.0),
            };
            (m.with_claim(v, u), pstar)
        })
}

/// `B(c)` per configuration with each parameter at the chosen value.
fn pstar_values(m: &MScenario, pstar: &[f64]) -> Vec<f64> {
    m.base
        .bound_values()
        .into_iter()
        .map(|b| pstar.iter().fold(b, |acc, p| acc * p))
        .collect()
}

proptest! {
    #![proptest_config(config())]

    /// P-02 (permissive variant), R-09: if some value inside the plausible
    /// ranges makes the claim possible for some configuration, the verdict is
    /// not IMPOSSIBLE.
    #[test]
    fn p02_never_impossible_when_a_plausible_value_makes_the_claim_possible(
        (m, pstar) in p02_scenario(),
    ) {
        let (lo, hi) = m.base.claim_interval();
        let (rel, abs) = m.base.guard.rel_abs();
        let possible = pstar_values(&m, &pstar).into_iter().any(|b| match m.base.direction {
            Direction::Max => lo <= b + g(b, rel, abs),
            Direction::Min => hi >= b - g(b, rel, abs),
        });
        if possible {
            let d = m.decide();
            prop_assert_ne!(d.verdict, Verdict::Impossible);
        }
    }

    /// P-03 (b), R-05, D-03, D-16, D-19, D-22, E-04: with missing parameters
    /// the engine agrees with the oracle on the verdict, on `C*` and on the
    /// `Certain` / `Depends` tags, and the payloads are consistent.
    #[test]
    fn p03b_e04_engine_matches_the_oracle_with_missing_parameters(m in m_scenario()) {
        let d = m.decide();
        let missing = m.missing();
        let missing_ids: Vec<ParameterId> = missing.iter().map(|(id, _)| id.clone()).collect();

        if m.any_no_domain() {
            // D-22: InsufficientData directly, all missing listed, no enclosures.
            prop_assert!(d.enclosures.is_none());
            match &d.verdict {
                Verdict::InsufficientData { missing: got } => {
                    let got: Vec<_> = got.iter().map(|x| (x.id.clone(), x.status)).collect();
                    prop_assert_eq!(got, missing);
                }
                other => prop_assert!(false, "expected InsufficientData, got {:?}", other),
            }
            return Ok(());
        }

        let observed = observe(&d, &missing_ids);
        prop_assert!(observed.is_ok(), "{:?}", observed);
        prop_assert_eq!(observed.unwrap_or(Expect::Insufficient), expect(&m));

        let enc = d.enclosures.expect("enclosures when every missing parameter has a domain");
        let ends = m.ends();
        let fold = |f: fn(f64, f64) -> f64, init: f64, pick: fn(&(f64, f64)) -> f64| {
            ends.iter().map(pick).fold(init, f)
        };
        prop_assert_eq!(enc.permissive.bounds.b_min, fold(f64::min, f64::INFINITY, |e| e.0));
        prop_assert_eq!(enc.permissive.bounds.b_max, fold(f64::max, f64::NEG_INFINITY, |e| e.0));
        prop_assert_eq!(enc.restrictive.bounds.b_min, fold(f64::min, f64::INFINITY, |e| e.1));
        prop_assert_eq!(enc.restrictive.bounds.b_max, fold(f64::max, f64::NEG_INFINITY, |e| e.1));
        for e in [enc.permissive, enc.restrictive] {
            if let Some(margin) = e.margin {
                prop_assert!(margin.low <= margin.high, "margin {margin:?}");
            }
        }
        match &d.verdict {
            Verdict::InsufficientData { missing: got } => {
                prop_assert!(!got.is_empty());
                let got: Vec<_> = got.iter().map(|x| (x.id.clone(), x.status)).collect();
                prop_assert_eq!(got, m.missing());
            }
            Verdict::Conditional { satisfying } => prop_assert!(!satisfying.is_empty()),
            Verdict::Impossible | Verdict::Compatible => {}
        }
    }

    /// E-02 with missing parameters: more uncertainty never creates IMPOSSIBLE
    /// or COMPATIBLE.
    #[test]
    fn e02_more_uncertainty_with_missing_parameters(m in m_scenario(), du in 0u32..=50) {
        let wider = m.with_claim(m.base.v, m.base.u + f64::from(du));
        let narrow = m.decide().verdict;
        let wide = wider.decide().verdict;
        if narrow != Verdict::Impossible {
            prop_assert_ne!(&wide, &Verdict::Impossible, "wider claim: {:?}", wider);
        }
        if narrow != Verdict::Compatible {
            prop_assert_ne!(&wide, &Verdict::Compatible, "wider claim: {:?}", wider);
        }
    }
}

/// Guard against an accidentally trivial generator: the claim modes must
/// produce all four kinds of outcome over the fixed-seed run.
#[test]
fn generator_reaches_every_outcome_with_missing_parameters() {
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::new(config());
    let strategy = m_scenario();
    let mut seen = [false; 4];
    for _ in 0..config().cases {
        let m = strategy.new_tree(&mut runner).expect("value").current();
        if m.any_no_domain() {
            continue;
        }
        match expect(&m) {
            Expect::Impossible => seen[0] = true,
            Expect::Compatible => seen[1] = true,
            Expect::Conditional(c) if c.iter().any(|(_, certain)| !certain) => seen[2] = true,
            Expect::Insufficient => seen[3] = true,
            Expect::Conditional(_) => {}
        }
    }
    assert_eq!(seen, [true; 4], "outcomes reached: {seen:?}");
}
