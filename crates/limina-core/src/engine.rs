//! The decision engine (design 6). Pure functions: no I/O, no hidden state.

use std::collections::BTreeMap;

use crate::config::ConfigurationSet;
use crate::error::EngineError;
use crate::interval::Interval;
use crate::limit::{
    Configuration, Direction, DomainRange, Limit, ParameterEnclosure, ParameterId, PlausibleDomain,
};
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
/// A missing parameter without a plausible domain gives `InsufficientData`
/// directly, with no enclosures. Otherwise the rules run under the permissive
/// and the restrictive enclosure; equal outcomes are returned as is and
/// different outcomes give `InsufficientData` (D-03), so `Impossible` needs
/// both enclosures to say `Impossible`. Members of `C*` are `Certain` when
/// the restrictive end also admits the claim, else `Depends` on all missing
/// parameters (D-19).
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
    // Step 4 (D-16, D-22): any `NoDomain` parameter ends the evaluation, with
    // all missing parameters listed, no enclosure and no call to `Limit::bound`.
    if missing.iter().any(|m| m.status == MissingStatus::NoDomain) {
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
    let (rel, abs) = (cfg.guard_rel(), effective_guard_abs(cfg, limit)?);
    let outcome_perm = rules(direction, lo, hi, &values_of(&permissive), rel, abs);
    let outcome_restr = rules(direction, lo, hi, &values_of(&restrictive), rel, abs);
    // Step 8 (D-03, R-05.3, R-05.4, R-05.6): equal outcomes are returned as is,
    // different outcomes give `InsufficientData` with all missing parameters.
    // `Impossible` therefore needs `Impossible` under both enclosures.
    let verdict = if outcome_perm != outcome_restr {
        Verdict::InsufficientData { missing }
    } else {
        match outcome_perm {
            Outcome::Impossible => Verdict::Impossible,
            Outcome::Compatible => Verdict::Compatible,
            // Step 9.
            Outcome::Conditional => Verdict::Conditional {
                satisfying: tagged_configs(
                    direction,
                    (lo, hi),
                    &permissive,
                    &restrictive,
                    &missing,
                    (rel, abs),
                ),
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

/// Hull `[min, max]` of a plausible domain, from its endpoints only (R-08.7).
/// `None` when the domain has no usable value (an empty finite set).
fn plausible_hull(domain: &PlausibleDomain) -> Result<Option<Interval>, EngineError> {
    match &domain.range {
        DomainRange::Interval(interval) => Ok(Some(*interval)),
        DomainRange::Finite(values) => {
            if values.iter().any(|x| !x.is_finite()) {
                return Err(EngineError::NonFiniteValue {
                    field: "plausible domain",
                });
            }
            let lo = values.iter().copied().reduce(f64::min);
            let hi = values.iter().copied().reduce(f64::max);
            match (lo, hi) {
                (Some(lo), Some(hi)) => Interval::new(lo, hi).map(Some),
                _ => Ok(None),
            }
        }
    }
}

/// Step 3: supplied parameters as point intervals, the missing required
/// parameters with a usable plausible domain as the hull of that domain, and
/// the list of all missing parameters tagged `NoDomain` or `HasDomain`, in
/// declaration order.
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
    let mut missing = Vec::new();
    for spec in specs
        .iter()
        .filter(|s| s.required && !known.contains_key(&s.id))
    {
        let hull = match &spec.plausible {
            Some(domain) => plausible_hull(domain)?,
            None => None,
        };
        let status = match hull {
            Some(hull) => {
                values.insert(spec.id.clone(), hull);
                MissingStatus::HasDomain
            }
            None => MissingStatus::NoDomain,
        };
        missing.push(MissingParameter {
            id: spec.id.clone(),
            status,
        });
    }
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

/// Absolute term of the guard for `limit` (D-24): the larger of the
/// settings' `guard_abs` and the limit's optional floor `Limit::guard_abs`.
///
/// Errors: `InvalidGuard` if the limit's floor is not finite or `< 0`.
fn effective_guard_abs(
    cfg: crate::settings::DecisionConfig,
    limit: &dyn Limit,
) -> Result<f64, EngineError> {
    let floor = limit.guard_abs();
    if !floor.is_finite() || floor < 0.0 {
        return Err(EngineError::InvalidGuard);
    }
    Ok(cfg.guard_abs().max(floor))
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

/// `C*` with `Certain` / `Depends` tags (design 6.4 step 9, R-05.7, R-05.8,
/// D-10, D-13, D-19).
///
/// Membership uses the permissive end (as `satisfying_configs`). A member is
/// `Certain` when the same guarded comparison holds on the restrictive end,
/// otherwise `Depends` on all missing parameters. Without missing parameters
/// both ends are equal, so every member is `Certain`.
///
/// Max-bound: `L + g(L) >= lo`. Min-bound: `V - g(V) <= hi`.
fn tagged_configs(
    direction: Direction,
    (lo, hi): (f64, f64),
    permissive: &[(Configuration, f64)],
    restrictive: &[(Configuration, f64)],
    missing: &[MissingParameter],
    (guard_rel, guard_abs): (f64, f64),
) -> Vec<SatisfyingConfig> {
    let depends: Vec<ParameterId> = missing.iter().map(|m| m.id.clone()).collect();
    let members = satisfying_configs(direction, lo, hi, permissive, guard_rel, guard_abs);
    let restrictive_members =
        satisfying_configs(direction, lo, hi, restrictive, guard_rel, guard_abs);
    members
        .into_iter()
        .map(|member| {
            let certain = restrictive_members
                .iter()
                .any(|r| r.config == member.config);
            SatisfyingConfig {
                config: member.config,
                support: if certain {
                    Support::Certain
                } else {
                    Support::Depends(depends.clone())
                },
            }
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
    /// Default absolute guard since D-24.
    const DEFAULT_ABS: f64 = 0.0;
    /// Absolute floor of the pre-D-24 default, kept as an explicit mode.
    const FLOOR_ABS: f64 = 1e-12;

    /// Independent guard: `max(rel * |x|, abs)`.
    fn g(x: f64, rel: f64, abs: f64) -> f64 {
        f64::max(rel * x.abs(), abs)
    }

    /// Guard modes: zero, default, large, extreme (all `rel < 1`).
    const MODES: [(f64, f64); 5] = [
        (0.0, 0.0),
        (DEFAULT_REL, DEFAULT_ABS),
        (DEFAULT_REL, FLOOR_ABS),
        (0.5, FLOOR_ABS),
        (0.999, FLOOR_ABS),
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

    // T28 (D-03, R-05.3, R-05.4, R-05.6): combine the two enclosures.

    fn domain(lo: f64, hi: f64) -> PlausibleDomain {
        PlausibleDomain {
            range: crate::limit::DomainRange::Interval(Interval::new(lo, hi).expect("valid")),
            source: "test".to_string(),
        }
    }

    fn p_missing() -> Verdict {
        Verdict::InsufficientData {
            missing: vec![MissingParameter {
                id: ParameterId::new("p"),
                status: MissingStatus::HasDomain,
            }],
        }
    }

    /// `B(a) = 5 p`, `B(b) = 8 p`, `p` missing in `[lo, hi]`.
    fn two_modes(direction: Direction, lo: f64, hi: f64) -> FakeLimit {
        FakeLimit::new(direction, &[("a", 5.0), ("b", 8.0)])
            .with_parameter("p", Some(domain(lo, hi)))
    }

    fn verdict_of(limit: &FakeLimit, v: f64) -> Verdict {
        decide(limit, &input(v, None)).expect("valid").verdict
    }

    #[test]
    fn max_bound_same_verdict_under_both_enclosures_is_returned_as_is() {
        // p in [1, 2]: permissive {10, 16}, restrictive {5, 8}.
        let limit = two_modes(Direction::Max, 1.0, 2.0);
        assert_eq!(verdict_of(&limit, 17.0), Verdict::Impossible);
        assert_eq!(verdict_of(&limit, 4.0), Verdict::Compatible);
        // p in [1, 1.1]: permissive {5.5, 8.8}, restrictive {5, 8}: both
        // CONDITIONAL for a claim of 6.
        let narrow = two_modes(Direction::Max, 1.0, 1.1);
        assert!(matches!(
            verdict_of(&narrow, 6.0),
            Verdict::Conditional { .. }
        ));
    }

    #[test]
    fn max_bound_different_verdicts_give_insufficient_data_with_all_missing() {
        let limit = two_modes(Direction::Max, 1.0, 2.0);
        // Permissive COMPATIBLE (9 <= 10), restrictive IMPOSSIBLE (9 > 8).
        assert_eq!(verdict_of(&limit, 9.0), p_missing());
        // Permissive CONDITIONAL (10 < 12 <= 16), restrictive IMPOSSIBLE.
        assert_eq!(verdict_of(&limit, 12.0), p_missing());
        // Permissive COMPATIBLE (6 <= 10), restrictive CONDITIONAL (5 < 6 <= 8).
        assert_eq!(verdict_of(&limit, 6.0), p_missing());
    }

    #[test]
    fn max_bound_impossible_under_the_restrictive_enclosure_only_is_not_impossible() {
        let limit = two_modes(Direction::Max, 1.0, 2.0);
        let d = decide(&limit, &input(9.0, None)).expect("valid");
        let e = d.enclosures.expect("both enclosures were computed");
        // The restrictive enclosure alone would say IMPOSSIBLE (9 > 8).
        assert!(9.0 > e.restrictive.bounds.b_max);
        assert_eq!(d.verdict, p_missing());
    }

    #[test]
    fn min_bound_same_verdict_under_both_enclosures_is_returned_as_is() {
        // p in [1, 2]: permissive (lower ends) {5, 8}, restrictive {10, 16}.
        let limit = two_modes(Direction::Min, 1.0, 2.0);
        assert_eq!(verdict_of(&limit, 4.0), Verdict::Impossible);
        assert_eq!(verdict_of(&limit, 17.0), Verdict::Compatible);
        // p in [1, 1.1]: permissive {5, 8}, restrictive {5.5, 8.8}: both
        // CONDITIONAL for a claim of 6.
        let narrow = two_modes(Direction::Min, 1.0, 1.1);
        assert!(matches!(
            verdict_of(&narrow, 6.0),
            Verdict::Conditional { .. }
        ));
    }

    #[test]
    fn min_bound_different_verdicts_give_insufficient_data_with_all_missing() {
        let limit = two_modes(Direction::Min, 1.0, 2.0);
        // Permissive COMPATIBLE (9 >= 8), restrictive IMPOSSIBLE (9 < 10).
        assert_eq!(verdict_of(&limit, 9.0), p_missing());
        // Permissive CONDITIONAL (5 <= 7 < 8), restrictive IMPOSSIBLE (7 < 10).
        assert_eq!(verdict_of(&limit, 7.0), p_missing());
    }

    #[test]
    fn disagreement_lists_every_missing_parameter() {
        let limit = FakeLimit::new(Direction::Max, &[("a", 5.0)])
            .with_parameter("p", Some(domain(1.0, 2.0)))
            .with_parameter("q", Some(domain(1.0, 3.0)));
        // B in [5, 30]: permissive 30, restrictive 5. Claim 20: COMPATIBLE
        // under the permissive end (20 <= 30), IMPOSSIBLE under the other.
        let d = decide(&limit, &input(20.0, None)).expect("valid");
        let ids_and_status: Vec<(&str, MissingStatus)> = match &d.verdict {
            Verdict::InsufficientData { missing } => {
                missing.iter().map(|m| (m.id.as_str(), m.status)).collect()
            }
            other => panic!("expected InsufficientData, got {other:?}"),
        };
        assert_eq!(
            ids_and_status,
            vec![
                ("p", MissingStatus::HasDomain),
                ("q", MissingStatus::HasDomain)
            ]
        );
        assert!(d.enclosures.is_some());
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

/// T27: the two enclosures with one missing parameter (steps 5 and 6).
///
/// Expected values are written out by hand from the intervals of the fake
/// limit, not computed with the engine's `select`.
#[cfg(test)]
mod enclosure_tests {
    use std::collections::BTreeMap;

    use super::decide;
    use crate::config::ConstraintsMap;
    use crate::dimension::Dimension;
    use crate::error::EngineError;
    use crate::interval::Interval;
    use crate::limit::{
        Assumption, AssumptionId, AssumptionOption, Configuration, Direction, DomainRange, Limit,
        OptionId, ParameterEnclosure, ParameterId, ParameterSpec, PlausibleDomain,
    };
    use crate::quantity::{Measure, MeasureKind, Quantity};
    use crate::settings::DecisionConfig;
    use crate::verdict::{BoundRange, Claim, Decision, DecisionInput, Margin};

    const DIMLESS: MeasureKind = MeasureKind::Plain(Dimension::DIMENSIONLESS);

    /// `B(mode) = base(mode) * p`, with `p` missing and plausible in `[2, 4]`.
    /// Bound intervals: a = [10, 20], b = [16, 32].
    struct OneParamLimit {
        direction: Direction,
        assumptions: Vec<Assumption>,
        parameters: Vec<ParameterSpec>,
    }

    impl OneParamLimit {
        fn new(direction: Direction) -> OneParamLimit {
            OneParamLimit {
                direction,
                assumptions: vec![Assumption {
                    id: AssumptionId::new("mode"),
                    domain: ["a", "b"]
                        .iter()
                        .map(|o| AssumptionOption {
                            id: OptionId::new(*o),
                            value: None,
                        })
                        .collect(),
                }],
                parameters: vec![ParameterSpec {
                    id: ParameterId::new("p"),
                    kind: DIMLESS,
                    required: true,
                    plausible: Some(PlausibleDomain {
                        range: DomainRange::Interval(Interval::new(2.0, 4.0).expect("valid")),
                        source: "test".to_string(),
                    }),
                }],
            }
        }
    }

    impl Limit for OneParamLimit {
        fn id(&self) -> &str {
            "one-param"
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
            let base = match config
                .option(&AssumptionId::new("mode"))
                .map(|o| o.as_str().to_string())
                .as_deref()
            {
                Some("a") => 5.0,
                Some("b") => 8.0,
                _ => return Err(EngineError::UnknownAssumption("mode".to_string())),
            };
            let p = params
                .get(&ParameterId::new("p"))
                .ok_or_else(|| EngineError::UnknownParameter("p".to_string()))?;
            Interval::point(base)?.mul(p)
        }
    }

    fn run(direction: Direction, v: f64) -> Decision {
        let input = DecisionInput {
            claim: Claim {
                value: Measure::Plain(Quantity::from_si(v, Dimension::DIMENSIONLESS).expect("ok")),
                uncertainty: None,
            },
            config: DecisionConfig::default(),
            constraints: ConstraintsMap::new(),
            known_parameters: BTreeMap::new(),
        };
        decide(&OneParamLimit::new(direction), &input).expect("valid")
    }

    fn range(b_min: f64, b_max: f64) -> BoundRange {
        BoundRange { b_min, b_max }
    }

    #[test]
    fn max_bound_permissive_is_upper_ends_restrictive_is_lower_ends() {
        let e = run(Direction::Max, 15.0)
            .enclosures
            .expect("HasDomain only");
        // Permissive: a -> 20, b -> 32. Restrictive: a -> 10, b -> 16.
        assert_eq!(e.permissive.bounds, range(20.0, 32.0));
        assert_eq!(e.restrictive.bounds, range(10.0, 16.0));
    }

    #[test]
    fn min_bound_permissive_is_lower_ends_restrictive_is_upper_ends() {
        let e = run(Direction::Min, 15.0)
            .enclosures
            .expect("HasDomain only");
        // Permissive: a -> 10, b -> 16. Restrictive: a -> 20, b -> 32.
        assert_eq!(e.permissive.bounds, range(10.0, 16.0));
        assert_eq!(e.restrictive.bounds, range(20.0, 32.0));
    }

    #[test]
    fn each_enclosure_carries_its_own_margin() {
        let e = run(Direction::Max, 15.0)
            .enclosures
            .expect("HasDomain only");
        // Claim is the point 15. Permissive: [15/32, 15/20]. Restrictive: [15/16, 15/10].
        assert_eq!(
            e.permissive.margin,
            Some(Margin {
                low: 15.0 / 32.0,
                high: 15.0 / 20.0
            })
        );
        assert_eq!(
            e.restrictive.margin,
            Some(Margin {
                low: 15.0 / 16.0,
                high: 15.0 / 10.0
            })
        );
    }

    #[test]
    fn enclosures_are_distinct_when_a_parameter_is_missing() {
        let e = run(Direction::Min, 15.0)
            .enclosures
            .expect("HasDomain only");
        assert_ne!(e.permissive, e.restrictive);
    }
}

/// T26: parameter resolution and the `NoDomain` short-circuit (steps 3, 4).
#[cfg(test)]
mod resolution_tests {
    use std::cell::Cell;
    use std::collections::BTreeMap;

    use super::{decide, resolve_parameters};
    use crate::config::ConstraintsMap;
    use crate::dimension::Dimension;
    use crate::error::EngineError;
    use crate::interval::Interval;
    use crate::limit::{
        Assumption, AssumptionId, AssumptionOption, Configuration, Direction, DomainRange, Limit,
        OptionId, ParameterEnclosure, ParameterId, ParameterSpec, PlausibleDomain,
    };
    use crate::quantity::{Measure, MeasureKind, Quantity};
    use crate::settings::DecisionConfig;
    use crate::verdict::{Claim, DecisionInput, MissingParameter, MissingStatus, Verdict};

    const DIMLESS: MeasureKind = MeasureKind::Plain(Dimension::DIMENSIONLESS);

    /// Fake limit for these tests: counts the calls to `bound`.
    struct CountingLimit {
        parameters: Vec<ParameterSpec>,
        assumptions: Vec<Assumption>,
        calls: Cell<usize>,
    }

    impl Limit for CountingLimit {
        fn id(&self) -> &str {
            "counting"
        }
        fn direction(&self) -> Direction {
            Direction::Max
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
            _config: &Configuration,
            _params: &ParameterEnclosure,
        ) -> Result<Interval, EngineError> {
            self.calls.set(self.calls.get() + 1);
            Interval::point(5.0)
        }
    }

    fn domain(range: DomainRange) -> Option<PlausibleDomain> {
        Some(PlausibleDomain {
            range,
            source: "test".to_string(),
        })
    }

    fn interval(lo: f64, hi: f64) -> DomainRange {
        DomainRange::Interval(Interval::new(lo, hi).expect("valid"))
    }

    fn spec(id: &str, required: bool, plausible: Option<PlausibleDomain>) -> ParameterSpec {
        ParameterSpec {
            id: ParameterId::new(id),
            kind: DIMLESS,
            required,
            plausible,
        }
    }

    fn limit(parameters: Vec<ParameterSpec>) -> CountingLimit {
        CountingLimit {
            parameters,
            assumptions: vec![Assumption {
                id: AssumptionId::new("mode"),
                domain: vec![AssumptionOption {
                    id: OptionId::new("a"),
                    value: None,
                }],
            }],
            calls: Cell::new(0),
        }
    }

    fn input(v: f64) -> DecisionInput {
        DecisionInput {
            claim: Claim {
                value: Measure::Plain(Quantity::from_si(v, Dimension::DIMENSIONLESS).expect("ok")),
                uncertainty: None,
            },
            config: DecisionConfig::default(),
            constraints: ConstraintsMap::new(),
            known_parameters: BTreeMap::new(),
        }
    }

    fn missing(id: &str, status: MissingStatus) -> MissingParameter {
        MissingParameter {
            id: ParameterId::new(id),
            status,
        }
    }

    #[test]
    fn one_no_domain_and_one_has_domain_list_both_without_enclosures() {
        // Declared order q, p: the list keeps declaration order.
        let l = limit(vec![
            spec("q", true, domain(interval(1.0, 2.0))),
            spec("p", true, None),
        ]);
        let d = decide(&l, &input(1e9)).expect("valid");
        assert_eq!(
            d.verdict,
            Verdict::InsufficientData {
                missing: vec![
                    missing("q", MissingStatus::HasDomain),
                    missing("p", MissingStatus::NoDomain),
                ]
            }
        );
        assert_eq!(d.enclosures, None);
        assert_eq!(l.calls.get(), 0, "Limit::bound must not be called");
    }

    #[test]
    fn two_no_domain_parameters_are_all_listed() {
        let l = limit(vec![spec("a", true, None), spec("b", true, None)]);
        let d = decide(&l, &input(0.0)).expect("valid");
        assert_eq!(
            d.verdict,
            Verdict::InsufficientData {
                missing: vec![
                    missing("a", MissingStatus::NoDomain),
                    missing("b", MissingStatus::NoDomain),
                ]
            }
        );
        assert_eq!(d.enclosures, None);
        assert_eq!(l.calls.get(), 0);
    }

    #[test]
    fn supplied_and_optional_parameters_are_not_missing() {
        let l = limit(vec![
            spec("known", true, None),
            spec("opt", false, None),
            spec("gap", true, domain(interval(1.0, 2.0))),
        ]);
        let mut known = BTreeMap::new();
        let m = Measure::Plain(Quantity::from_si(3.0, Dimension::DIMENSIONLESS).expect("ok"));
        known.insert(ParameterId::new("known"), m);
        let (enc, list) = resolve_parameters(&l, &known).expect("valid");
        assert_eq!(list, vec![missing("gap", MissingStatus::HasDomain)]);
        let p = enc.get(&ParameterId::new("known")).expect("supplied");
        assert_eq!((p.lo(), p.hi()), (3.0, 3.0));
        assert_eq!(enc.get(&ParameterId::new("opt")), None);
    }

    #[test]
    fn interval_domain_becomes_its_hull_in_constant_time() {
        let l = limit(vec![spec("p", true, domain(interval(0.0, 1e300)))]);
        let (enc, list) = resolve_parameters(&l, &BTreeMap::new()).expect("valid");
        assert_eq!(list, vec![missing("p", MissingStatus::HasDomain)]);
        let p = enc.get(&ParameterId::new("p")).expect("hull");
        assert_eq!((p.lo(), p.hi()), (0.0, 1e300));
    }

    #[test]
    fn finite_domain_becomes_min_max_hull() {
        let range = DomainRange::Finite(vec![3.0, -1.0, 2.0, 7.5]);
        let l = limit(vec![spec("p", true, domain(range))]);
        let (enc, _) = resolve_parameters(&l, &BTreeMap::new()).expect("valid");
        let p = enc.get(&ParameterId::new("p")).expect("hull");
        assert_eq!((p.lo(), p.hi()), (-1.0, 7.5));
    }

    #[test]
    fn empty_finite_domain_counts_as_no_domain() {
        let l = limit(vec![spec("p", true, domain(DomainRange::Finite(vec![])))]);
        let (enc, list) = resolve_parameters(&l, &BTreeMap::new()).expect("valid");
        assert_eq!(list, vec![missing("p", MissingStatus::NoDomain)]);
        assert_eq!(enc.get(&ParameterId::new("p")), None);
        let d = decide(&l, &input(1.0)).expect("valid");
        assert_eq!(d.enclosures, None);
        assert_eq!(l.calls.get(), 0);
    }

    #[test]
    fn no_domain_parameter_has_no_value_in_the_enclosure() {
        let l = limit(vec![spec("p", true, None)]);
        let (enc, _) = resolve_parameters(&l, &BTreeMap::new()).expect("valid");
        assert_eq!(enc.get(&ParameterId::new("p")), None);
    }
}

/// T29: `Certain` / `Depends` tags with missing parameters (R-05.7, R-05.8,
/// D-10, D-13, D-19). The limit and the oracle are local to this module.
#[cfg(test)]
mod support_tests {
    use std::collections::BTreeMap;

    use super::decide;
    use crate::config::ConstraintsMap;
    use crate::dimension::Dimension;
    use crate::error::EngineError;
    use crate::interval::Interval;
    use crate::limit::{
        Assumption, AssumptionId, AssumptionOption, Configuration, Direction, DomainRange, Limit,
        OptionId, ParameterEnclosure, ParameterId, ParameterSpec, PlausibleDomain,
    };
    use crate::quantity::{Measure, MeasureKind, Quantity};
    use crate::settings::DecisionConfig;
    use crate::verdict::{Claim, DecisionInput, Support, Verdict};

    const DIMLESS: MeasureKind = MeasureKind::Plain(Dimension::DIMENSIONLESS);

    /// `B(mode) = base(mode) * product of the missing parameters`. No physics.
    struct Lim {
        direction: Direction,
        assumptions: Vec<Assumption>,
        parameters: Vec<ParameterSpec>,
        base: BTreeMap<String, f64>,
    }

    fn lim(direction: Direction, base: &[(&str, f64)], params: &[(&str, f64, f64)]) -> Lim {
        Lim {
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
            parameters: params
                .iter()
                .map(|(id, lo, hi)| ParameterSpec {
                    id: ParameterId::new(*id),
                    kind: DIMLESS,
                    required: true,
                    plausible: Some(PlausibleDomain {
                        range: DomainRange::Interval(Interval::new(*lo, *hi).expect("valid")),
                        source: "test".to_string(),
                    }),
                })
                .collect(),
            base: base.iter().map(|(o, b)| (o.to_string(), *b)).collect(),
        }
    }

    impl Limit for Lim {
        fn id(&self) -> &str {
            "support-fake"
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

    fn claim(v: f64) -> DecisionInput {
        DecisionInput {
            claim: Claim {
                value: Measure::Plain(
                    Quantity::from_si(v, Dimension::DIMENSIONLESS).expect("valid"),
                ),
                uncertainty: None,
            },
            config: DecisionConfig::default(),
            constraints: ConstraintsMap::new(),
            known_parameters: BTreeMap::new(),
        }
    }

    /// C* as `(option, support)` pairs, in configuration order.
    fn tags(limit: &Lim, v: f64) -> Vec<(String, Support)> {
        match decide(limit, &claim(v)).expect("valid").verdict {
            Verdict::Conditional { satisfying } => satisfying
                .into_iter()
                .map(|s| {
                    let option = s
                        .config
                        .option(&AssumptionId::new("mode"))
                        .map(|o| o.as_str().to_string())
                        .unwrap_or_default();
                    (option, s.support)
                })
                .collect(),
            other => panic!("expected Conditional, got {other:?}"),
        }
    }

    fn depends(names: &[&str]) -> Support {
        Support::Depends(names.iter().map(|n| ParameterId::new(*n)).collect())
    }

    /// Independent guard `max(1e-9 * |x|, 0)` (the default settings, D-24).
    fn g(x: f64) -> f64 {
        f64::max(1e-9 * x.abs(), 0.0)
    }

    fn tag_of(tags: &[(String, Support)], option: &str) -> Option<Support> {
        tags.iter()
            .find(|(o, _)| o == option)
            .map(|(_, s)| s.clone())
    }

    #[test]
    fn max_bound_certain_and_depends() {
        // p in [0.8, 1.2]. Permissive (upper) ends: a 4.8, b 12, c 8.4.
        // Restrictive (lower) ends: a 3.2, b 8, c 5.6. Claim 6: both
        // enclosures CONDITIONAL.
        let limit = lim(
            Direction::Max,
            &[("a", 4.0), ("b", 10.0), ("c", 7.0)],
            &[("p", 0.8, 1.2)],
        );
        assert_eq!(
            tags(&limit, 6.0),
            vec![
                ("b".to_string(), Support::Certain),
                ("c".to_string(), depends(&["p"])),
            ]
        );
    }

    #[test]
    fn min_bound_certain_and_depends() {
        // p in [0.8, 1.2]. Permissive (lower) ends: a 3.2, b 8, c 5.6.
        // Restrictive (upper) ends: a 4.8, b 12, c 8.4. Claim 6.
        let limit = lim(
            Direction::Min,
            &[("a", 4.0), ("b", 10.0), ("c", 7.0)],
            &[("p", 0.8, 1.2)],
        );
        assert_eq!(
            tags(&limit, 6.0),
            vec![
                ("a".to_string(), Support::Certain),
                ("c".to_string(), depends(&["p"])),
            ]
        );
    }

    #[test]
    fn depends_lists_all_missing_parameters_in_declaration_order() {
        // q is a point domain: B(c) does not really depend on it (D-13), and
        // it is still listed.
        let limit = lim(
            Direction::Max,
            &[("a", 4.0), ("b", 10.0), ("c", 7.0)],
            &[("p", 0.8, 1.2), ("q", 1.0, 1.0)],
        );
        assert_eq!(tag_of(&tags(&limit, 6.0), "c"), Some(depends(&["p", "q"])));
    }

    #[test]
    fn conditional_with_missing_parameters_is_never_empty() {
        let limit = lim(
            Direction::Max,
            &[("a", 4.0), ("b", 10.0), ("c", 7.0)],
            &[("p", 0.8, 1.2)],
        );
        for v in [4.9, 6.0, 8.0, 11.0] {
            if let Verdict::Conditional { satisfying } =
                decide(&limit, &claim(v)).expect("valid").verdict
            {
                assert!(!satisfying.is_empty(), "claim {v}");
            }
        }
    }

    /// Max-bound at the guard boundary of the restrictive end. p in [1, 2],
    /// so the restrictive end of mode m is exactly 100 and the claim sits at
    /// `L + g(L)`. Modes z (50) and w (150) keep both enclosures CONDITIONAL.
    fn max_edge_limit() -> Lim {
        lim(
            Direction::Max,
            &[("m", 100.0), ("w", 150.0), ("z", 50.0)],
            &[("p", 1.0, 2.0)],
        )
    }

    #[test]
    fn max_bound_restrictive_end_exactly_at_the_guard_is_certain() {
        let edge = 100.0 + g(100.0);
        let t = tags(&max_edge_limit(), edge);
        assert_eq!(tag_of(&t, "m"), Some(Support::Certain));
        assert_eq!(tag_of(&t, "w"), Some(Support::Certain));
    }

    #[test]
    fn max_bound_one_ulp_above_the_restrictive_guard_is_depends() {
        let edge = (100.0 + g(100.0)).next_up();
        let t = tags(&max_edge_limit(), edge);
        assert_eq!(tag_of(&t, "m"), Some(depends(&["p"])));
        assert_eq!(tag_of(&t, "w"), Some(Support::Certain));
    }

    /// Min-bound at the guard boundary. p in [1, 2], so the restrictive
    /// (upper) end of mode m is exactly 200 and the claim sits at
    /// `V - g(V)`. Modes y (50) and z (400) keep both enclosures CONDITIONAL.
    fn min_edge_limit() -> Lim {
        lim(
            Direction::Min,
            &[("m", 100.0), ("y", 50.0), ("z", 400.0)],
            &[("p", 1.0, 2.0)],
        )
    }

    #[test]
    fn min_bound_restrictive_end_exactly_at_the_guard_is_certain() {
        let edge = 200.0 - g(200.0);
        let t = tags(&min_edge_limit(), edge);
        assert_eq!(tag_of(&t, "m"), Some(Support::Certain));
        assert_eq!(tag_of(&t, "y"), Some(Support::Certain));
    }

    #[test]
    fn min_bound_one_ulp_below_the_restrictive_guard_is_depends() {
        let edge = (200.0 - g(200.0)).next_down();
        let t = tags(&min_edge_limit(), edge);
        assert_eq!(tag_of(&t, "m"), Some(depends(&["p"])));
        assert_eq!(tag_of(&t, "y"), Some(Support::Certain));
    }
}
