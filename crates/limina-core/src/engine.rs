//! The decision engine (design 6). Pure functions: no I/O, no hidden state.

use std::collections::BTreeMap;

use crate::config::ConfigurationSet;
use crate::error::EngineError;
use crate::interval::Interval;
use crate::limit::{Configuration, Direction, Limit, ParameterEnclosure, ParameterId};
use crate::quantity::{check_same_kind, Measure, MeasureKind};
use crate::tolerance::guard;
use crate::verdict::{
    BoundRange, Claim, Decision, DecisionInput, EnclosureResult, Enclosures, Margin,
    MissingParameter, MissingStatus, SatisfyingConfig, Support, Verdict,
};

/// Which end of each `B(c)` range is used (R-05.2).
///
/// Max-bound: permissive is the upper end, restrictive the lower end.
/// Min-bound: permissive is the lower end, restrictive the upper end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnclosureKind {
    /// The end that makes the bound easiest to satisfy.
    Permissive,
    /// The end that makes the bound hardest to satisfy.
    Restrictive,
}

/// Computes the verdict of `input.claim` against `limit` (design 6.4).
///
/// This version handles claims without missing parameters. Until missing
/// parameters are handled (tasks T26 to T29), a missing required parameter
/// gives `InsufficientData` with all missing parameters and no enclosures,
/// which is never `Impossible`.
pub fn decide(limit: &dyn Limit, input: &DecisionInput) -> Result<Decision, EngineError> {
    let cfg = input.config;
    // Step 1.
    let (lo, hi) = claim_interval(limit.bounded_kind(), &input.claim, cfg.k())?;
    // Step 2.
    let set = ConfigurationSet::build(limit, &input.constraints, cfg.max_configurations())?;
    // Step 3.
    let (params, missing) = resolve_parameters(limit, &input.known_parameters)?;
    let decision = |verdict, enclosures| Decision {
        verdict,
        config: cfg,
        claim_lo: lo,
        claim_hi: hi,
        enclosures,
    };
    if !missing.is_empty() {
        // Temporary until T26 to T29: never IMPOSSIBLE.
        return Ok(decision(Verdict::InsufficientData { missing }, None));
    }
    // Step 5.
    let bounds: Vec<(Configuration, Interval)> = set
        .iter()
        .map(|c| checked_bound(limit, c, &params).map(|b| (c.clone(), b)))
        .collect::<Result<_, _>>()?;
    // Step 6.
    let direction = limit.direction();
    let permissive = selected(direction, EnclosureKind::Permissive, &bounds);
    let restrictive = selected(direction, EnclosureKind::Restrictive, &bounds);
    // Step 7.
    let (rel, abs) = (cfg.guard_rel(), cfg.guard_abs());
    let outcome_perm = rules(direction, lo, hi, &values_of(&permissive), rel, abs);
    let outcome_restr = rules(direction, lo, hi, &values_of(&restrictive), rel, abs);
    // Step 8 (D-03). With point bounds both outcomes are equal.
    let verdict = if outcome_perm != outcome_restr {
        Verdict::InsufficientData { missing }
    } else {
        match outcome_perm {
            Outcome::Impossible => Verdict::Impossible,
            Outcome::Compatible => Verdict::Compatible,
            // Step 9.
            Outcome::Conditional => Verdict::Conditional {
                satisfying: satisfying_configs(direction, lo, hi, &permissive, rel, abs),
            },
        }
    };
    // Step 10.
    let enclosures = Enclosures {
        permissive: enclosure_result(lo, hi, &permissive),
        restrictive: enclosure_result(lo, hi, &restrictive),
    };
    Ok(decision(verdict, Some(enclosures)))
}

/// Step 3: supplied parameters as point intervals, and the missing required
/// parameters tagged `NoDomain` or `HasDomain`, in declaration order.
fn resolve_parameters(
    limit: &dyn Limit,
    known: &BTreeMap<ParameterId, Measure>,
) -> Result<(ParameterEnclosure, Vec<MissingParameter>), EngineError> {
    let specs = limit.parameters();
    let mut values = BTreeMap::new();
    for (id, measure) in known {
        let spec = specs
            .iter()
            .find(|s| &s.id == id)
            .ok_or_else(|| EngineError::UnknownParameter(id.as_str().to_string()))?;
        check_same_kind(spec.kind, measure.kind())?;
        values.insert(id.clone(), Interval::point(measure.si_value())?);
    }
    let missing = specs
        .iter()
        .filter(|s| s.required && !known.contains_key(&s.id))
        .map(|s| MissingParameter {
            id: s.id.clone(),
            status: if s.plausible.is_some() {
                MissingStatus::HasDomain
            } else {
                MissingStatus::NoDomain
            },
        })
        .collect();
    Ok((ParameterEnclosure::new(values), missing))
}

/// The `enclosure` end of every `B(c)`.
fn selected(
    direction: Direction,
    enclosure: EnclosureKind,
    bounds: &[(Configuration, Interval)],
) -> Vec<(Configuration, f64)> {
    bounds
        .iter()
        .map(|(c, b)| (c.clone(), select(direction, enclosure, *b)))
        .collect()
}

fn values_of(values: &[(Configuration, f64)]) -> Vec<f64> {
    values.iter().map(|(_, b)| *b).collect()
}

/// `B_min`, `B_max` and margin of one enclosure.
fn enclosure_result(lo: f64, hi: f64, values: &[(Configuration, f64)]) -> EnclosureResult {
    let bs = values_of(values);
    let bounds = BoundRange {
        b_min: bs.iter().copied().fold(f64::INFINITY, f64::min),
        b_max: bs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    };
    EnclosureResult {
        bounds,
        margin: margin(lo, hi, bounds),
    }
}

/// Dispatches to the rules of the limit's direction.
fn rules(
    direction: Direction,
    lo: f64,
    hi: f64,
    bounds: &[f64],
    guard_rel: f64,
    guard_abs: f64,
) -> Outcome {
    match direction {
        Direction::Max => rules_max(lo, hi, bounds, guard_rel, guard_abs),
        Direction::Min => rules_min(lo, hi, bounds, guard_rel, guard_abs),
    }
}

/// `B_min` and `B_max` over `set`, using the `enclosure` end of each `B(c)`.
///
/// Errors: `InvalidBound` if a limit returns a negative bound (R-02.5);
/// any error returned by `Limit::bound` is propagated.
pub fn bound_range(
    limit: &dyn Limit,
    set: &ConfigurationSet,
    params: &ParameterEnclosure,
    enclosure: EnclosureKind,
) -> Result<BoundRange, EngineError> {
    if set.is_empty() {
        return Err(EngineError::EmptyConfigurationSet);
    }
    let mut b_min = f64::INFINITY;
    let mut b_max = f64::NEG_INFINITY;
    for config in set.iter() {
        let b = checked_bound(limit, config, params)?;
        let value = select(limit.direction(), enclosure, b);
        b_min = b_min.min(value);
        b_max = b_max.max(value);
    }
    Ok(BoundRange { b_min, b_max })
}

/// `B(c)` from the limit, rejected if negative. `Interval` already
/// guarantees finite endpoints.
fn checked_bound(
    limit: &dyn Limit,
    config: &Configuration,
    params: &ParameterEnclosure,
) -> Result<Interval, EngineError> {
    let b = limit.bound(config, params)?;
    if b.lo() < 0.0 {
        return Err(EngineError::InvalidBound {
            limit_id: limit.id().to_string(),
        });
    }
    Ok(b)
}

/// Step 1 of `decide`: checks the claim against the limit's bounded kind and
/// returns the claim interval `(lo, hi) = (v - k*u, v + k*u)` in SI base
/// units, with `u = 0` when absent (R-01.3, R-01.4, R-02.1, R-02.2, R-02.4).
///
/// `v` and `u` are finite by construction of `Measure`; `NonFiniteValue` is
/// returned if `k*u` or the interval overflows.
fn claim_interval(
    bounded_kind: MeasureKind,
    claim: &Claim,
    k: f64,
) -> Result<(f64, f64), EngineError> {
    let value_kind = claim.value.kind();
    check_same_kind(bounded_kind, value_kind)?;
    let v = claim.value.si_value();
    let u = match &claim.uncertainty {
        None => 0.0,
        Some(u) => {
            check_same_kind(value_kind.uncertainty_kind(), u.kind())?;
            if u.si_value() < 0.0 {
                return Err(EngineError::NegativeUncertainty);
            }
            u.si_value()
        }
    };
    let lo = v - k * u;
    let hi = v + k * u;
    if !lo.is_finite() || !hi.is_finite() {
        return Err(EngineError::NonFiniteValue {
            field: "claim interval",
        });
    }
    Ok((lo, hi))
}

/// Outcome of the three rules of R-03 / R-04 under one enclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Impossible,
    Conditional,
    Compatible,
}

/// Rules of R-03 (Max-bound) on the claim interval `[lo, hi]` and the bound
/// values `bounds` (one per configuration of `S`, under one enclosure).
///
/// - `Impossible` iff `lo > b + g(b)` for EVERY bound `b` (R-09, explicit
///   quantifier, design 6.4 step 7);
/// - else `Compatible` iff `hi <= B_min` (exact, no guard);
/// - else `Conditional`.
///
/// An empty `bounds` is never `Impossible`.
fn rules_max(lo: f64, hi: f64, bounds: &[f64], guard_rel: f64, guard_abs: f64) -> Outcome {
    if bounds.is_empty() {
        return Outcome::Conditional;
    }
    let g = |b: f64| guard(b, guard_rel, guard_abs);
    let impossible = bounds.iter().all(|&b| lo > b + g(b));
    let b_max = bounds.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // D-15 form; equal to the quantifier because x + g(x) is non-decreasing.
    debug_assert_eq!(impossible, lo > b_max + g(b_max));
    if impossible {
        return Outcome::Impossible;
    }
    let b_min = bounds.iter().copied().fold(f64::INFINITY, f64::min);
    if hi <= b_min {
        Outcome::Compatible
    } else {
        Outcome::Conditional
    }
}

/// Rules of R-04 (Min-bound), mirror of `rules_max`.
///
/// - `Impossible` iff `hi < b - g(b)` for EVERY bound `b`;
/// - else `Compatible` iff `lo >= B_max` (exact, no guard);
/// - else `Conditional`.
///
/// An empty `bounds` is never `Impossible`.
fn rules_min(lo: f64, hi: f64, bounds: &[f64], guard_rel: f64, guard_abs: f64) -> Outcome {
    if bounds.is_empty() {
        return Outcome::Conditional;
    }
    let g = |b: f64| guard(b, guard_rel, guard_abs);
    let impossible = bounds.iter().all(|&b| hi < b - g(b));
    let b_min = bounds.iter().copied().fold(f64::INFINITY, f64::min);
    // D-15 form; equal to the quantifier because guard_rel < 1 (D-18).
    debug_assert_eq!(impossible, hi < b_min - g(b_min));
    if impossible {
        return Outcome::Impossible;
    }
    let b_max = bounds.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if lo >= b_max {
        Outcome::Compatible
    } else {
        Outcome::Conditional
    }
}

/// `C*` without missing parameters (design 6.4 step 9, D-15): the
/// configurations whose bound, widened by the guard, admits the claim.
/// Every member is `Certain`.
///
/// Max-bound: `b(c) + g(b(c)) >= lo`. Min-bound: `b(c) - g(b(c)) <= hi`.
fn satisfying_configs(
    direction: Direction,
    lo: f64,
    hi: f64,
    values: &[(Configuration, f64)],
    guard_rel: f64,
    guard_abs: f64,
) -> Vec<SatisfyingConfig> {
    let g = |b: f64| guard(b, guard_rel, guard_abs);
    values
        .iter()
        .filter(|(_, b)| match direction {
            Direction::Max => b + g(*b) >= lo,
            Direction::Min => b - g(*b) <= hi,
        })
        .map(|(config, _)| SatisfyingConfig {
            config: config.clone(),
            support: Support::Certain,
        })
        .collect()
}

/// Margin (D-12): hull of `lo/B_max`, `lo/B_min`, `hi/B_max`, `hi/B_min`
/// when `B_min > 0`; `None` otherwise, without any division. `None` also
/// when any ratio is not finite (D-23). The verdict never depends on it.
fn margin(lo: f64, hi: f64, bounds: BoundRange) -> Option<Margin> {
    if bounds.b_min <= 0.0 {
        return None;
    }
    let ratios = [
        lo / bounds.b_max,
        lo / bounds.b_min,
        hi / bounds.b_max,
        hi / bounds.b_min,
    ];
    if ratios.iter().any(|r| !r.is_finite()) {
        return None;
    }
    Some(Margin {
        low: ratios.iter().copied().fold(f64::INFINITY, f64::min),
        high: ratios.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    })
}

/// The end of `b` used under `enclosure` for a limit of `direction`.
fn select(direction: Direction, enclosure: EnclosureKind, b: Interval) -> f64 {
    match (direction, enclosure) {
        (Direction::Max, EnclosureKind::Permissive) => b.hi(),
        (Direction::Max, EnclosureKind::Restrictive) => b.lo(),
        (Direction::Min, EnclosureKind::Permissive) => b.lo(),
        (Direction::Min, EnclosureKind::Restrictive) => b.hi(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{bound_range, EnclosureKind};
    use crate::config::{ConfigurationSet, ConstraintsMap};
    use crate::dimension::Dimension;
    use crate::error::EngineError;
    use crate::interval::Interval;
    use crate::limit::{
        Assumption, AssumptionId, AssumptionOption, Configuration, Direction, Limit, OptionId,
        ParameterEnclosure, ParameterSpec,
    };
    use crate::quantity::MeasureKind;
    use crate::verdict::BoundRange;

    /// Fake limit for these tests only: one assumption "mode", a table of
    /// bound intervals per option, no physics.
    struct FakeLimit {
        direction: Direction,
        assumptions: Vec<Assumption>,
        table: BTreeMap<String, (f64, f64)>,
    }

    impl FakeLimit {
        fn new(direction: Direction, table: &[(&str, f64, f64)]) -> FakeLimit {
            FakeLimit {
                direction,
                assumptions: vec![Assumption {
                    id: AssumptionId::new("mode"),
                    domain: table
                        .iter()
                        .map(|(o, _, _)| AssumptionOption {
                            id: OptionId::new(*o),
                            value: None,
                        })
                        .collect(),
                }],
                table: table
                    .iter()
                    .map(|(o, lo, hi)| (o.to_string(), (*lo, *hi)))
                    .collect(),
            }
        }
    }

    impl Limit for FakeLimit {
        fn id(&self) -> &str {
            "fake"
        }
        fn direction(&self) -> Direction {
            self.direction
        }
        fn bounded_kind(&self) -> MeasureKind {
            MeasureKind::Plain(Dimension::DIMENSIONLESS)
        }
        fn assumptions(&self) -> &[Assumption] {
            &self.assumptions
        }
        fn parameters(&self) -> &[ParameterSpec] {
            &[]
        }
        fn bound(
            &self,
            config: &Configuration,
            _: &ParameterEnclosure,
        ) -> Result<Interval, EngineError> {
            let option = config
                .option(&AssumptionId::new("mode"))
                .ok_or_else(|| EngineError::UnknownAssumption("mode".to_string()))?;
            let (lo, hi) = self
                .table
                .get(option.as_str())
                .copied()
                .ok_or_else(|| EngineError::UnknownAssumption("mode".to_string()))?;
            Interval::new(lo, hi)
        }
    }

    fn range(limit: &FakeLimit, enclosure: EnclosureKind) -> Result<BoundRange, EngineError> {
        let set = ConfigurationSet::build(limit, &ConstraintsMap::new(), 100)?;
        bound_range(limit, &set, &ParameterEnclosure::default(), enclosure)
    }

    #[test]
    fn point_bounds_give_the_same_range_under_both_enclosures() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0, 5.0), ("b", 8.0, 8.0)]);
        let expected = Ok(BoundRange {
            b_min: 5.0,
            b_max: 8.0,
        });
        assert_eq!(range(&limit, EnclosureKind::Permissive), expected);
        assert_eq!(range(&limit, EnclosureKind::Restrictive), expected);
    }

    #[test]
    fn max_bound_permissive_uses_upper_ends_restrictive_lower_ends() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 4.0, 6.0), ("b", 7.0, 9.0)]);
        assert_eq!(
            range(&limit, EnclosureKind::Permissive),
            Ok(BoundRange {
                b_min: 6.0,
                b_max: 9.0
            })
        );
        assert_eq!(
            range(&limit, EnclosureKind::Restrictive),
            Ok(BoundRange {
                b_min: 4.0,
                b_max: 7.0
            })
        );
    }

    #[test]
    fn min_bound_permissive_uses_lower_ends_restrictive_upper_ends() {
        let limit = FakeLimit::new(Direction::Min, &[("a", 4.0, 6.0), ("b", 7.0, 9.0)]);
        assert_eq!(
            range(&limit, EnclosureKind::Permissive),
            Ok(BoundRange {
                b_min: 4.0,
                b_max: 7.0
            })
        );
        assert_eq!(
            range(&limit, EnclosureKind::Restrictive),
            Ok(BoundRange {
                b_min: 6.0,
                b_max: 9.0
            })
        );
    }

    #[test]
    fn zero_bound_is_accepted() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 0.0, 0.0)]);
        assert_eq!(
            range(&limit, EnclosureKind::Permissive),
            Ok(BoundRange {
                b_min: 0.0,
                b_max: 0.0
            })
        );
    }

    #[test]
    fn negative_bound_is_rejected() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0, 5.0), ("b", -1.0, 2.0)]);
        assert_eq!(
            range(&limit, EnclosureKind::Permissive),
            Err(EngineError::InvalidBound {
                limit_id: "fake".to_string()
            })
        );
    }

    #[test]
    fn errors_from_the_limit_propagate() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0, 5.0)]);
        let set = ConfigurationSet::build(&limit, &ConstraintsMap::new(), 100).expect("valid set");
        let other = FakeLimit::new(Direction::Max, &[("zzz", 1.0, 1.0)]);
        // `other` has no entry for option "a" of `set`.
        assert_eq!(
            bound_range(
                &other,
                &set,
                &ParameterEnclosure::default(),
                EnclosureKind::Permissive
            ),
            Err(EngineError::UnknownAssumption("mode".to_string()))
        );
    }
}

/// T16: verdict rules at their exact boundaries (design 9.4).
///
/// The boundary values are computed with the test's own guard formula, not
/// with `crate::tolerance::guard`, so the tests do not share code with the
/// engine.
#[cfg(test)]
mod rules_tests {
    use super::{rules_max, rules_min, Outcome};

    const DEFAULT_REL: f64 = 1e-9;
    const DEFAULT_ABS: f64 = 1e-12;

    /// Independent guard: `max(rel * |x|, abs)`.
    fn g(x: f64, rel: f64, abs: f64) -> f64 {
        f64::max(rel * x.abs(), abs)
    }

    /// Guard modes: zero, default, large, extreme (all `rel < 1`).
    const MODES: [(f64, f64); 4] = [
        (0.0, 0.0),
        (DEFAULT_REL, DEFAULT_ABS),
        (0.5, DEFAULT_ABS),
        (0.999, DEFAULT_ABS),
    ];

    #[test]
    fn max_bound_impossible_starts_one_ulp_above_b_plus_g() {
        for (rel, abs) in MODES {
            for b in [0.0, 1.0, 100.0, 1e6] {
                let edge = b + g(b, rel, abs);
                assert_ne!(
                    rules_max(edge, edge, &[b], rel, abs),
                    Outcome::Impossible,
                    "v = b + g(b) must not be IMPOSSIBLE (b={b}, rel={rel}, abs={abs})"
                );
                assert_eq!(
                    rules_max(edge.next_up(), edge.next_up(), &[b], rel, abs),
                    Outcome::Impossible,
                    "one ulp above b + g(b) is IMPOSSIBLE (b={b}, rel={rel}, abs={abs})"
                );
                assert_ne!(
                    rules_max(edge.next_down(), edge.next_down(), &[b], rel, abs),
                    Outcome::Impossible
                );
            }
        }
    }

    #[test]
    fn min_bound_impossible_starts_one_ulp_below_b_minus_g() {
        for (rel, abs) in MODES {
            for b in [0.0, 1.0, 100.0, 1e6] {
                let edge = b - g(b, rel, abs);
                assert_ne!(
                    rules_min(edge, edge, &[b], rel, abs),
                    Outcome::Impossible,
                    "v = b - g(b) must not be IMPOSSIBLE (b={b}, rel={rel}, abs={abs})"
                );
                assert_eq!(
                    rules_min(edge.next_down(), edge.next_down(), &[b], rel, abs),
                    Outcome::Impossible,
                    "one ulp below b - g(b) is IMPOSSIBLE (b={b}, rel={rel}, abs={abs})"
                );
                assert_ne!(
                    rules_min(edge.next_up(), edge.next_up(), &[b], rel, abs),
                    Outcome::Impossible
                );
            }
        }
    }

    #[test]
    fn max_bound_compatible_is_exact_at_b_min() {
        for (rel, abs) in MODES {
            assert_eq!(
                rules_max(100.0, 100.0, &[100.0], rel, abs),
                Outcome::Compatible
            );
            let above = 100.0_f64.next_up();
            let expected = if rel == 0.0 && abs == 0.0 {
                Outcome::Impossible
            } else {
                Outcome::Conditional
            };
            assert_eq!(rules_max(above, above, &[100.0], rel, abs), expected);
        }
    }

    #[test]
    fn min_bound_compatible_is_exact_at_b_max() {
        for (rel, abs) in MODES {
            assert_eq!(
                rules_min(100.0, 100.0, &[100.0], rel, abs),
                Outcome::Compatible
            );
            let below = 100.0_f64.next_down();
            let expected = if rel == 0.0 && abs == 0.0 {
                Outcome::Impossible
            } else {
                Outcome::Conditional
            };
            assert_eq!(rules_min(below, below, &[100.0], rel, abs), expected);
        }
    }

    #[test]
    fn max_bound_with_several_configurations() {
        let bounds = [5.0, 8.0];
        let (rel, abs) = (0.0, 0.0);
        assert_eq!(rules_max(9.0, 9.0, &bounds, rel, abs), Outcome::Impossible);
        assert_eq!(rules_max(8.0, 8.0, &bounds, rel, abs), Outcome::Conditional);
        assert_eq!(rules_max(6.0, 6.0, &bounds, rel, abs), Outcome::Conditional);
        assert_eq!(rules_max(5.0, 5.0, &bounds, rel, abs), Outcome::Compatible);
        // Interval claims.
        assert_eq!(rules_max(4.0, 6.0, &bounds, rel, abs), Outcome::Conditional);
        assert_eq!(rules_max(3.0, 5.0, &bounds, rel, abs), Outcome::Compatible);
        assert_eq!(rules_max(9.0, 10.0, &bounds, rel, abs), Outcome::Impossible);
        // lo at the largest bound is not IMPOSSIBLE, whatever hi is.
        assert_eq!(
            rules_max(8.0, 20.0, &bounds, rel, abs),
            Outcome::Conditional
        );
    }

    #[test]
    fn min_bound_with_several_configurations() {
        let bounds = [5.0, 8.0];
        let (rel, abs) = (0.0, 0.0);
        assert_eq!(rules_min(4.0, 4.0, &bounds, rel, abs), Outcome::Impossible);
        assert_eq!(rules_min(5.0, 5.0, &bounds, rel, abs), Outcome::Conditional);
        assert_eq!(rules_min(7.0, 7.0, &bounds, rel, abs), Outcome::Conditional);
        assert_eq!(rules_min(8.0, 8.0, &bounds, rel, abs), Outcome::Compatible);
        assert_eq!(rules_min(7.0, 9.0, &bounds, rel, abs), Outcome::Conditional);
        assert_eq!(rules_min(8.0, 9.0, &bounds, rel, abs), Outcome::Compatible);
        assert_eq!(rules_min(1.0, 4.0, &bounds, rel, abs), Outcome::Impossible);
        // hi at the smallest bound is not IMPOSSIBLE, whatever lo is.
        assert_eq!(
            rules_min(-10.0, 5.0, &bounds, rel, abs),
            Outcome::Conditional
        );
    }

    #[test]
    fn empty_bound_list_is_never_impossible() {
        // Unreachable through `decide` (S is never empty), but the rules
        // must not turn "no configuration" into IMPOSSIBLE.
        assert_ne!(rules_max(1e9, 1e9, &[], 0.0, 0.0), Outcome::Impossible);
        assert_ne!(rules_min(-1e9, -1e9, &[], 0.0, 0.0), Outcome::Impossible);
    }
}

/// T19: `decide` end to end, without missing parameters.
#[cfg(test)]
mod decide_tests {
    use std::collections::BTreeMap;

    use super::decide;
    use crate::config::ConstraintsMap;
    use crate::dimension::Dimension;
    use crate::error::EngineError;
    use crate::interval::Interval;
    use crate::limit::{
        Assumption, AssumptionId, AssumptionOption, Configuration, Direction, Limit, OptionId,
        ParameterEnclosure, ParameterId, ParameterSpec, PlausibleDomain,
    };
    use crate::quantity::{Measure, MeasureKind, Quantity};
    use crate::settings::DecisionConfig;
    use crate::verdict::{
        BoundRange, Claim, DecisionInput, MissingParameter, MissingStatus, Support, Verdict,
    };

    const DIMLESS: MeasureKind = MeasureKind::Plain(Dimension::DIMENSIONLESS);

    /// Fake limit for these tests only: `B(mode) = base(mode) * p` when the
    /// parameter `p` is declared and supplied. No physics.
    struct FakeLimit {
        direction: Direction,
        assumptions: Vec<Assumption>,
        parameters: Vec<ParameterSpec>,
        base: BTreeMap<String, f64>,
    }

    impl FakeLimit {
        fn new(direction: Direction, base: &[(&str, f64)]) -> FakeLimit {
            FakeLimit {
                direction,
                assumptions: vec![Assumption {
                    id: AssumptionId::new("mode"),
                    domain: base
                        .iter()
                        .map(|(o, _)| AssumptionOption {
                            id: OptionId::new(*o),
                            value: None,
                        })
                        .collect(),
                }],
                parameters: Vec::new(),
                base: base.iter().map(|(o, b)| (o.to_string(), *b)).collect(),
            }
        }

        fn with_parameter(mut self, id: &str, plausible: Option<PlausibleDomain>) -> FakeLimit {
            self.parameters.push(ParameterSpec {
                id: ParameterId::new(id),
                kind: DIMLESS,
                required: true,
                plausible,
            });
            self
        }
    }

    impl Limit for FakeLimit {
        fn id(&self) -> &str {
            "fake"
        }
        fn direction(&self) -> Direction {
            self.direction
        }
        fn bounded_kind(&self) -> MeasureKind {
            DIMLESS
        }
        fn assumptions(&self) -> &[Assumption] {
            &self.assumptions
        }
        fn parameters(&self) -> &[ParameterSpec] {
            &self.parameters
        }
        fn bound(
            &self,
            config: &Configuration,
            params: &ParameterEnclosure,
        ) -> Result<Interval, EngineError> {
            let option = config
                .option(&AssumptionId::new("mode"))
                .ok_or_else(|| EngineError::UnknownAssumption("mode".to_string()))?;
            let base = self
                .base
                .get(option.as_str())
                .copied()
                .ok_or_else(|| EngineError::UnknownAssumption("mode".to_string()))?;
            let mut b = Interval::point(base)?;
            for spec in &self.parameters {
                if let Some(p) = params.get(&spec.id) {
                    b = b.mul(p)?;
                }
            }
            Ok(b)
        }
    }

    fn dimless(x: f64) -> Measure {
        Measure::Plain(Quantity::from_si(x, Dimension::DIMENSIONLESS).expect("valid"))
    }

    fn input(v: f64, u: Option<f64>) -> DecisionInput {
        DecisionInput {
            claim: Claim {
                value: dimless(v),
                uncertainty: u.map(dimless),
            },
            config: DecisionConfig::default(),
            constraints: ConstraintsMap::new(),
            known_parameters: BTreeMap::new(),
        }
    }

    fn c_star(verdict: &Verdict) -> Vec<String> {
        match verdict {
            Verdict::Conditional { satisfying } => satisfying
                .iter()
                .map(|s| {
                    assert_eq!(s.support, Support::Certain);
                    s.config
                        .option(&AssumptionId::new("mode"))
                        .map(|o| o.as_str().to_string())
                        .unwrap_or_default()
                })
                .collect(),
            other => panic!("expected Conditional, got {other:?}"),
        }
    }

    #[test]
    fn max_bound_gives_each_verdict() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0), ("b", 8.0)]);
        let d = decide(&limit, &input(9.0, None)).expect("valid");
        assert_eq!(d.verdict, Verdict::Impossible);
        let d = decide(&limit, &input(6.0, None)).expect("valid");
        assert_eq!(c_star(&d.verdict), vec!["b"]);
        let d = decide(&limit, &input(4.0, None)).expect("valid");
        assert_eq!(d.verdict, Verdict::Compatible);
    }

    #[test]
    fn min_bound_gives_each_verdict() {
        let limit = FakeLimit::new(Direction::Min, &[("a", 5.0), ("b", 8.0)]);
        let d = decide(&limit, &input(4.0, None)).expect("valid");
        assert_eq!(d.verdict, Verdict::Impossible);
        let d = decide(&limit, &input(6.0, None)).expect("valid");
        assert_eq!(c_star(&d.verdict), vec!["a"]);
        let d = decide(&limit, &input(9.0, None)).expect("valid");
        assert_eq!(d.verdict, Verdict::Compatible);
    }

    #[test]
    fn uncertainty_widens_the_claim_interval() {
        // v = 9 alone is IMPOSSIBLE; with u = 1, k = 2, lo = 7 <= 8.
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0), ("b", 8.0)]);
        let d = decide(&limit, &input(9.0, Some(1.0))).expect("valid");
        assert_eq!((d.claim_lo, d.claim_hi), (7.0, 11.0));
        assert_eq!(c_star(&d.verdict), vec!["b"]);
    }

    #[test]
    fn output_carries_equal_enclosures_and_the_settings_used() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0), ("b", 8.0)]);
        let d = decide(&limit, &input(4.0, None)).expect("valid");
        assert_eq!(d.config, DecisionConfig::default());
        let e = d.enclosures.expect("no missing parameter");
        assert_eq!(e.permissive, e.restrictive);
        assert_eq!(
            e.permissive.bounds,
            BoundRange {
                b_min: 5.0,
                b_max: 8.0
            }
        );
        let m = e.permissive.margin.expect("B_min > 0");
        assert_eq!((m.low, m.high), (4.0 / 8.0, 4.0 / 5.0));
    }

    #[test]
    fn constraints_narrow_s_before_the_verdict() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0), ("b", 8.0)]);
        let mut i = input(6.0, None);
        let only_a: std::collections::BTreeSet<OptionId> = [OptionId::new("a")].into();
        i.constraints.allow(AssumptionId::new("mode"), only_a);
        assert_eq!(
            decide(&limit, &i).expect("valid").verdict,
            Verdict::Impossible
        );
    }

    #[test]
    fn supplied_parameters_enter_the_bound() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0)]).with_parameter("p", None);
        let mut i = input(12.0, None);
        i.known_parameters
            .insert(ParameterId::new("p"), dimless(2.0));
        // B = 5 * 2 = 10 < 12.
        assert_eq!(
            decide(&limit, &i).expect("valid").verdict,
            Verdict::Impossible
        );
        i.known_parameters
            .insert(ParameterId::new("p"), dimless(3.0));
        // B = 15 >= 12.
        assert_eq!(
            decide(&limit, &i).expect("valid").verdict,
            Verdict::Compatible
        );
    }

    #[test]
    fn unknown_parameter_is_rejected() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0)]);
        let mut i = input(1.0, None);
        i.known_parameters
            .insert(ParameterId::new("zzz"), dimless(1.0));
        assert_eq!(
            decide(&limit, &i),
            Err(EngineError::UnknownParameter("zzz".to_string()))
        );
    }

    #[test]
    fn parameter_of_the_wrong_kind_is_rejected() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0)]).with_parameter("p", None);
        let mut i = input(1.0, None);
        let metre = Measure::Plain(Quantity::from_unit(1.0, "m").expect("valid"));
        i.known_parameters.insert(ParameterId::new("p"), metre);
        assert!(matches!(
            decide(&limit, &i),
            Err(EngineError::IncompatibleDimension { .. })
        ));
    }

    #[test]
    fn missing_parameter_gives_insufficient_data_without_enclosures() {
        let domain = PlausibleDomain {
            range: crate::limit::DomainRange::Interval(Interval::new(1.0, 2.0).expect("valid")),
            source: "test".to_string(),
        };
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0)])
            .with_parameter("p", None)
            .with_parameter("q", Some(domain));
        // Far above any bound: must still not be IMPOSSIBLE.
        let d = decide(&limit, &input(1e9, None)).expect("valid");
        assert_eq!(
            d.verdict,
            Verdict::InsufficientData {
                missing: vec![
                    MissingParameter {
                        id: ParameterId::new("p"),
                        status: MissingStatus::NoDomain,
                    },
                    MissingParameter {
                        id: ParameterId::new("q"),
                        status: MissingStatus::HasDomain,
                    },
                ]
            }
        );
        assert_eq!(d.enclosures, None);
    }

    #[test]
    fn empty_configuration_set_is_an_error_not_a_verdict() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0)]);
        let mut i = input(1.0, None);
        i.constraints
            .allow(AssumptionId::new("mode"), std::collections::BTreeSet::new());
        assert_eq!(decide(&limit, &i), Err(EngineError::EmptyConfigurationSet));
    }

    #[test]
    fn claim_of_the_wrong_kind_is_rejected() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0)]);
        let mut i = input(1.0, None);
        i.claim.value = Measure::Plain(Quantity::from_unit(1.0, "W").expect("valid"));
        assert!(matches!(
            decide(&limit, &i),
            Err(EngineError::IncompatibleDimension { .. })
        ));
    }
}

/// T18: margin as the hull of four ratios (D-09, D-12).
#[cfg(test)]
mod margin_tests {
    use super::margin;
    use crate::verdict::{BoundRange, Margin};

    fn range(b_min: f64, b_max: f64) -> BoundRange {
        BoundRange { b_min, b_max }
    }

    #[test]
    fn positive_claim_equals_lo_over_b_max_to_hi_over_b_min() {
        // D-09 form: [8/20, 12/10] = [0.4, 1.2].
        assert_eq!(
            margin(8.0, 12.0, range(10.0, 20.0)),
            Some(Margin {
                low: 8.0 / 20.0,
                high: 12.0 / 10.0
            })
        );
    }

    #[test]
    fn negative_claim_gives_an_ordered_interval() {
        // D-09 form would be [-1/10, -1/1] = [-0.1, -1], inverted.
        let m = margin(-1.0, -1.0, range(1.0, 10.0)).expect("B_min > 0");
        assert!(m.low <= m.high);
        assert_eq!(
            m,
            Margin {
                low: -1.0,
                high: -0.1
            }
        );
    }

    #[test]
    fn claim_straddling_zero_takes_the_hull_of_the_four_ratios() {
        // Ratios: -2/4, -2/1, 3/4, 3/1 -> [-2, 3].
        assert_eq!(
            margin(-2.0, 3.0, range(1.0, 4.0)),
            Some(Margin {
                low: -2.0,
                high: 3.0
            })
        );
    }

    #[test]
    fn zero_bound_gives_no_margin() {
        assert_eq!(margin(1.0, 2.0, range(0.0, 5.0)), None);
        assert_eq!(margin(0.0, 0.0, range(0.0, 0.0)), None);
    }

    #[test]
    fn non_finite_ratio_gives_no_margin() {
        // D-23: 1 / 1e-320 overflows to infinity.
        assert_eq!(margin(1.0, 2.0, range(1e-320, 1.0)), None);
        // f64::MAX / 0.5 overflows to infinity.
        assert_eq!(margin(f64::MAX, f64::MAX, range(0.5, 1.0)), None);
        // A tiny but harmless B_min still gives a margin.
        assert!(margin(1e-300, 1e-300, range(1e-300, 1.0)).is_some());
    }
}

/// T17: `C*` membership at the guard boundary, without missing parameters.
#[cfg(test)]
mod c_star_tests {
    use std::collections::BTreeMap;

    use super::{rules_max, rules_min, satisfying_configs, Outcome};
    use crate::limit::{AssumptionId, Configuration, Direction, OptionId};
    use crate::verdict::Support;

    /// Independent guard: `max(rel * |x|, abs)`.
    fn g(x: f64, rel: f64, abs: f64) -> f64 {
        f64::max(rel * x.abs(), abs)
    }

    fn config(option: &str) -> Configuration {
        let mut m = BTreeMap::new();
        m.insert(AssumptionId::new("mode"), OptionId::new(option));
        Configuration::from_map(m)
    }

    fn table(bounds: &[(&str, f64)]) -> Vec<(Configuration, f64)> {
        bounds.iter().map(|(o, b)| (config(o), *b)).collect()
    }

    fn member_options(
        direction: Direction,
        lo: f64,
        hi: f64,
        values: &[(Configuration, f64)],
        rel: f64,
        abs: f64,
    ) -> Vec<String> {
        satisfying_configs(direction, lo, hi, values, rel, abs)
            .into_iter()
            .map(|s| {
                assert_eq!(s.support, Support::Certain);
                s.config
                    .option(&AssumptionId::new("mode"))
                    .map(|o| o.as_str().to_string())
                    .unwrap_or_default()
            })
            .collect()
    }

    #[test]
    fn max_bound_membership_at_the_guard_boundary() {
        let (rel, abs) = (0.5, 1e-12);
        let values = table(&[("a", 100.0), ("b", 200.0)]);
        let edge_a = 100.0 + g(100.0, rel, abs); // 150
        assert_eq!(
            member_options(Direction::Max, edge_a, edge_a, &values, rel, abs),
            vec!["a", "b"]
        );
        let above = edge_a.next_up();
        assert_eq!(
            member_options(Direction::Max, above, above, &values, rel, abs),
            vec!["b"]
        );
    }

    #[test]
    fn min_bound_membership_at_the_guard_boundary() {
        let (rel, abs) = (0.5, 1e-12);
        let values = table(&[("a", 100.0), ("b", 200.0)]);
        let edge_b = 200.0 - g(200.0, rel, abs); // 100
        assert_eq!(
            member_options(Direction::Min, edge_b, edge_b, &values, rel, abs),
            vec!["a", "b"]
        );
        let below = edge_b.next_down();
        assert_eq!(
            member_options(Direction::Min, below, below, &values, rel, abs),
            vec!["a"]
        );
    }

    #[test]
    fn zero_guard_membership_is_exact() {
        let values = table(&[("a", 5.0), ("b", 8.0)]);
        assert_eq!(
            member_options(Direction::Max, 8.0, 8.0, &values, 0.0, 0.0),
            vec!["b"]
        );
        assert_eq!(
            member_options(Direction::Max, 5.0, 9.0, &values, 0.0, 0.0),
            vec!["a", "b"]
        );
        assert_eq!(
            member_options(Direction::Min, 5.0, 5.0, &values, 0.0, 0.0),
            vec!["a"]
        );
    }

    #[test]
    fn the_extreme_configuration_is_kept_whenever_not_impossible() {
        let values = table(&[("a", 3.0), ("b", 7.0), ("c", 5.0)]);
        let bounds: Vec<f64> = values.iter().map(|(_, b)| *b).collect();
        for (rel, abs) in [(0.0, 0.0), (1e-9, 1e-12), (0.5, 1e-12)] {
            for step in 0..=40 {
                let v = f64::from(step) * 0.25; // 0 to 10
                if rules_max(v, v, &bounds, rel, abs) != Outcome::Impossible {
                    let m = member_options(Direction::Max, v, v, &values, rel, abs);
                    assert!(m.contains(&"b".to_string()), "Max-bound v={v} rel={rel}");
                }
                if rules_min(v, v, &bounds, rel, abs) != Outcome::Impossible {
                    let m = member_options(Direction::Min, v, v, &values, rel, abs);
                    assert!(m.contains(&"a".to_string()), "Min-bound v={v} rel={rel}");
                }
            }
        }
    }
}

/// T15: claim validation and claim interval (step 1).
#[cfg(test)]
mod claim_tests {
    use super::claim_interval;
    use crate::dimension::Dimension;
    use crate::error::EngineError;
    use crate::quantity::{
        AbsoluteTemperature, Measure, MeasureKind, Quantity, TemperatureDifference,
    };
    use crate::verdict::Claim;

    const POWER: Dimension = Dimension::new([2, 1, -3, 0, 0, 0, 0]);
    const ENERGY: Dimension = Dimension::new([2, 1, -2, 0, 0, 0, 0]);

    fn plain(value: f64, unit: &str) -> Measure {
        Measure::Plain(Quantity::from_unit(value, unit).expect("valid quantity"))
    }

    fn claim(value: Measure, uncertainty: Option<Measure>) -> Claim {
        Claim { value, uncertainty }
    }

    #[test]
    fn interval_is_v_plus_minus_k_u_in_si() {
        let c = claim(plain(10.0, "kW"), Some(plain(500.0, "W")));
        assert_eq!(
            claim_interval(MeasureKind::Plain(POWER), &c, 2.0),
            Ok((9000.0, 11000.0))
        );
    }

    #[test]
    fn absent_uncertainty_means_u_zero() {
        let c = claim(plain(7.0, "W"), None);
        assert_eq!(
            claim_interval(MeasureKind::Plain(POWER), &c, 3.0),
            Ok((7.0, 7.0))
        );
    }

    #[test]
    fn claim_of_another_dimension_is_rejected() {
        let c = claim(plain(1.0, "kWh"), None);
        assert_eq!(
            claim_interval(MeasureKind::Plain(POWER), &c, 2.0),
            Err(EngineError::IncompatibleDimension {
                expected: POWER,
                found: ENERGY,
            })
        );
    }

    #[test]
    fn uncertainty_of_another_dimension_is_rejected() {
        let c = claim(plain(1.0, "kW"), Some(plain(1.0, "J")));
        assert_eq!(
            claim_interval(MeasureKind::Plain(POWER), &c, 2.0),
            Err(EngineError::IncompatibleDimension {
                expected: POWER,
                found: ENERGY,
            })
        );
    }

    #[test]
    fn absolute_temperature_claim_takes_a_difference_as_uncertainty() {
        let t =
            Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(300.0).expect("valid"));
        let du =
            Measure::TemperatureDifference(TemperatureDifference::from_kelvin(5.0).expect("valid"));
        let c = claim(t, Some(du));
        assert_eq!(
            claim_interval(MeasureKind::AbsoluteTemperature, &c, 2.0),
            Ok((290.0, 310.0))
        );

        // An absolute temperature is not a valid uncertainty.
        let bad = claim(t, Some(t));
        assert_eq!(
            claim_interval(MeasureKind::AbsoluteTemperature, &bad, 2.0),
            Err(EngineError::TemperatureKindMismatch)
        );
    }

    #[test]
    fn absolute_against_difference_claim_is_a_kind_mismatch() {
        let d =
            Measure::TemperatureDifference(TemperatureDifference::from_kelvin(5.0).expect("valid"));
        assert_eq!(
            claim_interval(MeasureKind::AbsoluteTemperature, &claim(d, None), 2.0),
            Err(EngineError::TemperatureKindMismatch)
        );
    }

    #[test]
    fn negative_uncertainty_is_rejected() {
        let c = claim(plain(1.0, "W"), Some(plain(-0.5, "W")));
        assert_eq!(
            claim_interval(MeasureKind::Plain(POWER), &c, 2.0),
            Err(EngineError::NegativeUncertainty)
        );
        // A temperature difference can be negative, but not as an uncertainty.
        let t =
            Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(300.0).expect("valid"));
        let du = Measure::TemperatureDifference(
            TemperatureDifference::from_kelvin(-1.0).expect("valid"),
        );
        assert_eq!(
            claim_interval(MeasureKind::AbsoluteTemperature, &claim(t, Some(du)), 2.0),
            Err(EngineError::NegativeUncertainty)
        );
    }

    #[test]
    fn overflowing_claim_interval_is_rejected() {
        let max = Measure::Plain(Quantity::from_si(f64::MAX, POWER).expect("valid"));
        let c = claim(max, Some(max));
        assert_eq!(
            claim_interval(MeasureKind::Plain(POWER), &c, 2.0),
            Err(EngineError::NonFiniteValue {
                field: "claim interval"
            })
        );
    }
}
