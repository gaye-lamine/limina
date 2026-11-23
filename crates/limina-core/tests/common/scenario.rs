//! Generated scenarios for the property tests, and an independent oracle.
//!
//! The oracle recomputes the guard with its own formula and decides
//! IMPOSSIBLE with the explicit quantifier over every configuration of `S`
//! (design 6.4 step 7). It shares no code with the engine.

use std::collections::BTreeMap;
use std::fmt;

use limina_core::{
    decide, Claim, ConstraintsMap, Decision, DecisionConfig, DecisionInput, Dimension, Direction,
    EngineError, Measure, Quantity, Verdict,
};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::TestLimit;

/// Fixed seed, so CI runs are reproducible (`testing.md`).
pub const SEED: u64 = 0x4C49_4D49_4E41; // "LIMINA"

/// Cases per property: 1024 in CI (the `CI` variable is set by GitHub
/// Actions), 256 locally.
pub fn cases() -> u32 {
    if std::env::var_os("CI").is_some() {
        1024
    } else {
        256
    }
}

/// Proptest configuration with the fixed seed.
pub fn config() -> Config {
    Config {
        cases: cases(),
        rng_seed: RngSeed::Fixed(SEED),
        ..Config::default()
    }
}

/// Guard modes of design 9.2: zero, default, large, extreme (all `rel < 1`),
/// plus the absolute floor of the pre-D-24 default (`abs = 1e-12`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GuardMode {
    Zero,
    Default,
    AbsFloor,
    Large(f64),
    Extreme(f64),
}

impl GuardMode {
    pub fn rel_abs(self) -> (f64, f64) {
        match self {
            GuardMode::Zero => (0.0, 0.0),
            GuardMode::Default => (1e-9, 0.0),
            GuardMode::AbsFloor => (1e-9, 1e-12),
            GuardMode::Large(rel) | GuardMode::Extreme(rel) => (rel, 1e-12),
        }
    }
}

/// One generated test case: a limit given by a table of point bounds, a
/// claim, and the engine settings.
#[derive(Clone)]
pub struct Scenario {
    pub direction: Direction,
    /// Number of options of each assumption (1 to 3 assumptions).
    pub dims: Vec<usize>,
    /// Bound of each configuration, keyed by its option ids in assumption
    /// order. One entry per configuration of `S`.
    pub bounds: BTreeMap<Vec<String>, f64>,
    pub v: f64,
    pub u: f64,
    pub k: f64,
    pub guard: GuardMode,
}

impl fmt::Debug for Scenario {
    /// Readable one-line form, printed by proptest for a shrunk failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dir = match self.direction {
            Direction::Max => "Max-bound",
            Direction::Min => "Min-bound",
        };
        let bounds: Vec<String> = self
            .bounds
            .iter()
            .map(|(key, b)| format!("B({})={b:?}", key.join(",")))
            .collect();
        let (rel, abs) = self.guard.rel_abs();
        let (lo, hi) = self.claim_interval();
        write!(
            f,
            "{dir} | {} | claim v={:?} u={:?} k={:?} -> lo={lo:?} hi={hi:?} | guard {:?} (rel={rel:e}, abs={abs:e})",
            bounds.join(", "),
            self.v,
            self.u,
            self.k,
            self.guard,
        )
    }
}

impl Scenario {
    /// `(v - k*u, v + k*u)`, the oracle's own computation.
    pub fn claim_interval(&self) -> (f64, f64) {
        (self.v - self.k * self.u, self.v + self.k * self.u)
    }

    /// Bound values in key order.
    pub fn bound_values(&self) -> Vec<f64> {
        self.bounds.values().copied().collect()
    }

    pub fn limit(&self) -> TestLimit {
        let mut limit = TestLimit::new(self.direction);
        for (i, n) in self.dims.iter().enumerate() {
            let options: Vec<String> = (0..*n).map(|o| o.to_string()).collect();
            let refs: Vec<&str> = options.iter().map(String::as_str).collect();
            limit = limit.with_assumption(&format!("a{i}"), &refs);
        }
        for (key, b) in &self.bounds {
            let refs: Vec<&str> = key.iter().map(String::as_str).collect();
            limit = limit.with_bound(&refs, *b, *b);
        }
        limit
    }

    pub fn input(&self) -> DecisionInput {
        let (rel, abs) = self.guard.rel_abs();
        let dimless = |x: f64| {
            Measure::Plain(Quantity::from_si(x, Dimension::DIMENSIONLESS).expect("finite value"))
        };
        DecisionInput {
            claim: Claim {
                value: dimless(self.v),
                uncertainty: Some(dimless(self.u)),
            },
            config: DecisionConfig::new(self.k, rel, abs, 100_000).expect("valid settings"),
            constraints: ConstraintsMap::new(),
            known_parameters: BTreeMap::new(),
        }
    }

    pub fn decide(&self) -> Result<Decision, EngineError> {
        decide(&self.limit(), &self.input())
    }

    /// Same scenario with another claim.
    pub fn with_claim(&self, v: f64, u: f64) -> Scenario {
        Scenario {
            v,
            u,
            ..self.clone()
        }
    }
}

/// Verdict computed by the oracle; `C*` as option-id keys.
#[derive(Clone, Debug, PartialEq)]
pub enum Expected {
    Impossible,
    Conditional(Vec<Vec<String>>),
    Compatible,
}

/// Independent guard: `max(rel * |x|, abs)`.
pub fn g(x: f64, rel: f64, abs: f64) -> f64 {
    f64::max(rel * x.abs(), abs)
}

/// The three predicates of R-03 / R-04, computed by brute force.
pub struct Predicates {
    pub impossible: bool,
    pub compatible: bool,
    pub c_star: Vec<Vec<String>>,
}

pub fn predicates(s: &Scenario) -> Predicates {
    let (lo, hi) = s.claim_interval();
    let (rel, abs) = s.guard.rel_abs();
    let values = s.bound_values();
    let b_min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let b_max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (impossible, compatible) = match s.direction {
        Direction::Max => (
            s.bounds.values().all(|&b| lo > b + g(b, rel, abs)),
            hi <= b_min,
        ),
        Direction::Min => (
            s.bounds.values().all(|&b| hi < b - g(b, rel, abs)),
            lo >= b_max,
        ),
    };
    let c_star = s
        .bounds
        .iter()
        .filter(|(_, &b)| match s.direction {
            Direction::Max => b + g(b, rel, abs) >= lo,
            Direction::Min => b - g(b, rel, abs) <= hi,
        })
        .map(|(key, _)| key.clone())
        .collect();
    Predicates {
        impossible,
        compatible,
        c_star,
    }
}

/// Oracle verdict (R-03 / R-04 with the guard of D-15).
pub fn oracle(s: &Scenario) -> Expected {
    let p = predicates(s);
    if p.impossible {
        Expected::Impossible
    } else if p.compatible {
        Expected::Compatible
    } else {
        Expected::Conditional(p.c_star)
    }
}

/// The engine verdict in the oracle's form. Panics on `InsufficientData`,
/// which cannot occur without parameters.
pub fn observed(verdict: &Verdict) -> Expected {
    match verdict {
        Verdict::Impossible => Expected::Impossible,
        Verdict::Compatible => Expected::Compatible,
        Verdict::Conditional { satisfying } => Expected::Conditional(
            satisfying
                .iter()
                .map(|s| {
                    s.config
                        .iter()
                        .map(|(_, o)| o.as_str().to_string())
                        .collect()
                })
                .collect(),
        ),
        Verdict::InsufficientData { missing } => {
            panic!("unexpected InsufficientData without parameters: {missing:?}")
        }
    }
}

/// All configuration keys for `dims`, in lexicographic order.
fn keys(dims: &[usize]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = vec![Vec::new()];
    for n in dims {
        out = out
            .into_iter()
            .flat_map(|prefix| {
                (0..*n).map(move |o| {
                    let mut next = prefix.clone();
                    next.push(o.to_string());
                    next
                })
            })
            .collect();
    }
    out
}

pub fn guard_mode() -> impl Strategy<Value = GuardMode> {
    prop_oneof![
        Just(GuardMode::Zero),
        Just(GuardMode::Default),
        Just(GuardMode::AbsFloor),
        (1e-3..=0.5f64).prop_map(GuardMode::Large),
        (0.9..=0.999f64).prop_map(GuardMode::Extreme),
    ]
}

/// How the claim value is placed.
#[derive(Clone, Copy, Debug)]
enum ClaimMode {
    /// Any integer in `0..=2000`.
    Free(u32),
    /// A chosen bound plus a small integer offset.
    NearBound(usize, i32),
    /// A chosen bound's guard edge (`b + g(b)` or `b - g(b)`), plus `-1`,
    /// `0` or `+1` ulp; `u = 0`.
    GuardEdge(usize, i8),
}

fn claim_mode() -> impl Strategy<Value = ClaimMode> {
    prop_oneof![
        (0u32..=2000).prop_map(ClaimMode::Free),
        (0usize..27, -2i32..=2).prop_map(|(i, o)| ClaimMode::NearBound(i, o)),
        (0usize..27, -1i8..=1).prop_map(|(i, s)| ClaimMode::GuardEdge(i, s)),
    ]
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

/// Scenarios without missing parameters: 1 to 3 assumptions with 1 to 3
/// options, integer bounds in `0..=1000`, the claim modes above, `u` in
/// `0..=50`, `k` in `{1, 2, 3}`, and the four guard modes.
pub fn scenario() -> impl Strategy<Value = Scenario> {
    (
        any::<bool>(),
        proptest::collection::vec(1usize..=3, 1..=3),
        proptest::collection::vec(0u32..=1000, 27),
        claim_mode(),
        0u32..=50,
        1u32..=3,
        guard_mode(),
    )
        .prop_map(|(is_max, dims, raw, mode, u, k, guard)| {
            let direction = if is_max {
                Direction::Max
            } else {
                Direction::Min
            };
            let all_keys = keys(&dims);
            let bounds: BTreeMap<Vec<String>, f64> = all_keys
                .into_iter()
                .zip(raw.iter().map(|b| f64::from(*b)))
                .collect();
            let values: Vec<f64> = bounds.values().copied().collect();
            let (rel, abs) = guard.rel_abs();
            let (v, u) = match mode {
                ClaimMode::Free(x) => (f64::from(x), f64::from(u)),
                ClaimMode::NearBound(i, o) => {
                    (values[i % values.len()] + f64::from(o), f64::from(u))
                }
                ClaimMode::GuardEdge(i, step) => (
                    edge(direction, values[i % values.len()], rel, abs, step),
                    0.0,
                ),
            };
            Scenario {
                direction,
                dims,
                bounds,
                v,
                u,
                k: f64::from(k),
                guard,
            }
        })
}
