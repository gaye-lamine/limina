//! Inverse questions: the value of one parameter at which the bound equals
//! the claim.
//!
//! The bound, as a function of the free parameter with every other parameter
//! fixed, is the largest bound of the configuration set for a Max limit (a
//! claim is IMPOSSIBLE only above it) and the smallest for a Min limit. Its
//! monotonicity is established first, with the sampling and trend rules of
//! the check report; a non-monotone or undetermined function gives no
//! threshold. The threshold is then found by bisection, and the verdicts
//! just below and above it come from the decision engine.

use std::collections::BTreeMap;

use limina_core::{
    decide, AbsoluteTemperature, Claim, ConfigurationSet, ConstraintsMap, DecisionConfig,
    DecisionInput, Direction, DomainRange, EngineError, Interval, Limit, Measure, MeasureKind,
    ParameterEnclosure, ParameterId, Quantity, TemperatureDifference, Verdict,
};

use crate::limit::DeclarativeLimit;
use crate::report::{samples, trend, Monotonicity, MONOTONICITY_SAMPLES};

/// Relative offset of the points just below and just above the threshold.
pub const SIDE_OFFSET: f64 = 1e-6;
/// Largest number of configurations considered.
const MAX_CONFIGURATIONS: usize = 1024;

/// Which side of the threshold the parameter must be on for the claim not to
/// be inconsistent with the limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdDirection {
    /// The parameter must be at least the threshold.
    AtLeast,
    /// The parameter must be at most the threshold.
    AtMost,
}

impl ThresholdDirection {
    /// "at least" or "at most".
    pub fn as_str(self) -> &'static str {
        match self {
            ThresholdDirection::AtLeast => "at least",
            ThresholdDirection::AtMost => "at most",
        }
    }
}

/// The question. Values are in SI units, temperatures in kelvin.
#[derive(Debug, Clone, Default)]
pub struct SolveRequest {
    /// Claimed value.
    pub claim: f64,
    /// Claimed uncertainty, if any.
    pub uncertainty: Option<f64>,
    /// Allowed options per assumption.
    pub constraints: ConstraintsMap,
    /// Every other parameter, fixed.
    pub fixed: BTreeMap<String, f64>,
    /// The parameter to solve for.
    pub free: String,
    /// Search range; None uses the plausible domain, else the test range.
    pub range: Option<(f64, f64)>,
}

/// The answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Solution {
    /// Value of the free parameter at which the bound equals the claim.
    pub threshold: f64,
    /// Side the parameter must be on.
    pub direction: ThresholdDirection,
    /// First unit declared for the free parameter.
    pub unit: String,
    /// Search range used.
    pub range: (f64, f64),
    /// Monotonicity of the bound in the free parameter on the range.
    pub monotonicity: Monotonicity,
    /// Point just below the threshold.
    pub below: f64,
    /// Point just above the threshold.
    pub above: f64,
    /// Engine verdict at `below`.
    pub verdict_below: Verdict,
    /// Engine verdict at `above`.
    pub verdict_above: Verdict,
    /// "Not inconsistent with limit X only if P is at least V (...)".
    pub sentence: String,
}

/// Why no threshold is given.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SolveError {
    /// The free parameter is not declared by the limit.
    #[error("the limit has no parameter {0}")]
    UnknownFreeParameter(String),
    /// A declared parameter other than the free one has no fixed value.
    #[error("parameter {0} needs a fixed value")]
    MissingFixedParameter(String),
    /// No plausible domain and no test range for the free parameter.
    #[error("no search range for {0}: declare a plausible domain or a test range")]
    NoSearchRange(String),
    /// The bound is not monotone in the free parameter on the range.
    #[error("the bound is not monotone in {parameter} on the search range ({monotonicity:?}): no threshold is given")]
    NotMonotone {
        /// The free parameter.
        parameter: String,
        /// What the sampling found.
        monotonicity: Monotonicity,
    },
    /// The bound never equals the claim on the range.
    #[error("the bound does not reach the claim for {parameter} in [{lo}, {hi}]")]
    NoThresholdInRange {
        /// The free parameter.
        parameter: String,
        /// Lower end of the range.
        lo: f64,
        /// Upper end of the range.
        hi: f64,
    },
    /// The engine refused the input.
    #[error(transparent)]
    Engine(#[from] EngineError),
}

fn measure(kind: MeasureKind, v: f64) -> Result<Measure, EngineError> {
    Ok(match kind {
        MeasureKind::AbsoluteTemperature => {
            Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(v)?)
        }
        MeasureKind::TemperatureDifference => {
            Measure::TemperatureDifference(TemperatureDifference::from_kelvin(v)?)
        }
        MeasureKind::Plain(d) => Measure::Plain(Quantity::from_si(v, d)?),
    })
}

/// A number with up to six significant digits and no trailing zeros.
fn number(v: f64) -> String {
    if v != 0.0 && (v.abs() >= 1e6 || v.abs() < 1e-3) {
        return format!("{v:.5e}");
    }
    let digits = (5 - v.abs().log10().floor().max(0.0) as i32).max(0) as usize;
    let s = format!("{v:.digits$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

fn with_unit(v: f64, unit: &str) -> String {
    if unit.is_empty() || unit == "dimensionless" {
        number(v)
    } else {
        format!("{} {unit}", number(v))
    }
}

struct Problem<'a> {
    limit: &'a DeclarativeLimit,
    set: ConfigurationSet,
    request: &'a SolveRequest,
}

impl Problem<'_> {
    /// The bound that decides IMPOSSIBLE at this value of the free parameter.
    fn bound_at(&self, x: f64) -> Result<f64, EngineError> {
        let mut map = BTreeMap::new();
        for (name, v) in &self.request.fixed {
            map.insert(ParameterId::new(name.clone()), Interval::point(*v)?);
        }
        map.insert(
            ParameterId::new(self.request.free.clone()),
            Interval::point(x)?,
        );
        let enclosure = ParameterEnclosure::new(map);
        let mut out: Option<f64> = None;
        for config in self.set.iter() {
            let b = self.limit.bound(config, &enclosure)?;
            out = Some(match (self.limit.direction(), out) {
                (Direction::Max, None) => b.hi(),
                (Direction::Max, Some(o)) => o.max(b.hi()),
                (Direction::Min, None) => b.lo(),
                (Direction::Min, Some(o)) => o.min(b.lo()),
            });
        }
        out.ok_or(EngineError::EmptyConfigurationSet)
    }

    fn verdict_at(&self, x: f64) -> Result<Verdict, EngineError> {
        let MeasureKind::Plain(dim) = self.limit.bounded_kind() else {
            return Err(EngineError::TemperatureKindRequired);
        };
        let mut known = BTreeMap::new();
        for spec in self.limit.parameters() {
            let name = spec.id.as_str();
            let v = if name == self.request.free {
                Some(x)
            } else {
                self.request.fixed.get(name).copied()
            };
            if let Some(v) = v {
                known.insert(spec.id.clone(), measure(spec.kind, v)?);
            }
        }
        let input = DecisionInput {
            claim: Claim {
                value: Measure::Plain(Quantity::from_si(self.request.claim, dim)?),
                uncertainty: self
                    .request
                    .uncertainty
                    .map(|u| Quantity::from_si(u, dim).map(Measure::Plain))
                    .transpose()?,
            },
            config: DecisionConfig::default(),
            constraints: self.request.constraints.clone(),
            known_parameters: known,
        };
        Ok(decide(self.limit, &input)?.verdict)
    }
}

/// The search range: the request's, else the plausible domain, else the
/// test range.
fn search_range(
    limit: &DeclarativeLimit,
    request: &SolveRequest,
) -> Result<(f64, f64), SolveError> {
    if let Some(r) = request.range {
        return Ok(r);
    }
    let spec = limit
        .parameters()
        .iter()
        .find(|p| p.id.as_str() == request.free);
    if let Some(domain) = spec.and_then(|s| s.plausible.as_ref()) {
        return Ok(match &domain.range {
            DomainRange::Interval(i) => (i.lo(), i.hi()),
            DomainRange::Finite(v) => (
                v.iter().copied().fold(f64::INFINITY, f64::min),
                v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ),
        });
    }
    limit
        .definition()
        .parameters
        .iter()
        .find(|p| p.name == request.free)
        .and_then(|p| p.test_range)
        .map(|[lo, hi]| (lo, hi))
        .ok_or_else(|| SolveError::NoSearchRange(request.free.clone()))
}

/// Finds the value of `request.free` at which the bound equals the claim.
pub fn solve_for(limit: &DeclarativeLimit, request: &SolveRequest) -> Result<Solution, SolveError> {
    let free = request.free.clone();
    let declared = limit
        .definition()
        .parameters
        .iter()
        .find(|p| p.name == free);
    let Some(declared) = declared else {
        return Err(SolveError::UnknownFreeParameter(free));
    };
    for p in &limit.definition().parameters {
        if p.name != free && !request.fixed.contains_key(&p.name) {
            return Err(SolveError::MissingFixedParameter(p.name.clone()));
        }
    }
    let (lo, hi) = search_range(limit, request)?;
    let set = ConfigurationSet::build(limit, &request.constraints, MAX_CONFIGURATIONS)?;
    let problem = Problem {
        limit,
        set,
        request,
    };

    // Monotonicity first, with the report's rules; skipped points are where
    // the bound is undefined (a validity condition fails).
    let points: Vec<(f64, f64)> = samples(lo, hi, MONOTONICITY_SAMPLES)
        .into_iter()
        .filter_map(|x| problem.bound_at(x).ok().map(|b| (x, b)))
        .collect();
    let monotonicity = trend(&points.iter().map(|(_, b)| *b).collect::<Vec<_>>());
    if !matches!(
        monotonicity,
        Monotonicity::Increasing | Monotonicity::Decreasing
    ) {
        return Err(SolveError::NotMonotone {
            parameter: free,
            monotonicity,
        });
    }

    // A sign change of bound - claim between two neighbouring samples.
    let claim = request.claim;
    let g = |b: f64| b - claim;
    let bracket = points
        .windows(2)
        .find(|w| g(w[0].1) == 0.0 || g(w[0].1).signum() != g(w[1].1).signum())
        .map(|w| (w[0], w[1]));
    let Some(((mut a, ga), (mut b, _))) = bracket else {
        return Err(SolveError::NoThresholdInRange {
            parameter: free,
            lo,
            hi,
        });
    };
    let threshold = if g(ga) == 0.0 {
        a
    } else {
        let negative_at_a = g(ga) < 0.0;
        for _ in 0..2000 {
            let mid = a + (b - a) / 2.0;
            if mid <= a || mid >= b {
                break;
            }
            let gm = g(problem.bound_at(mid)?);
            if gm == 0.0 {
                a = mid;
                b = mid;
                break;
            }
            if (gm < 0.0) == negative_at_a {
                a = mid;
            } else {
                b = mid;
            }
        }
        a + (b - a) / 2.0
    };

    let increasing = monotonicity == Monotonicity::Increasing;
    let direction = match (limit.direction(), increasing) {
        (Direction::Max, true) | (Direction::Min, false) => ThresholdDirection::AtLeast,
        _ => ThresholdDirection::AtMost,
    };
    let offset = if threshold == 0.0 {
        SIDE_OFFSET
    } else {
        threshold.abs() * SIDE_OFFSET
    };
    let (below, above) = (threshold - offset, threshold + offset);
    let unit = declared.units.first().cloned().unwrap_or_default();
    let others: Vec<String> = limit
        .definition()
        .parameters
        .iter()
        .filter(|p| p.name != free)
        .filter_map(|p| {
            let v = request.fixed.get(&p.name)?;
            Some(format!(
                "{} = {}",
                p.name,
                with_unit(*v, p.units.first().map_or("", String::as_str))
            ))
        })
        .collect();
    let fixed = if others.is_empty() {
        "no other parameter".to_string()
    } else {
        format!("other parameters fixed at {}", others.join(", "))
    };
    let sentence = format!(
        "Not inconsistent with limit {} only if {free} is {} {} ({fixed})",
        limit.id(),
        direction.as_str(),
        with_unit(threshold, &unit),
    );
    Ok(Solution {
        threshold,
        direction,
        unit,
        range: (lo, hi),
        monotonicity,
        below,
        above,
        verdict_below: problem.verdict_at(below)?,
        verdict_above: problem.verdict_at(above)?,
        sentence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_short_and_readable() {
        assert_eq!(number(750.0), "750");
        assert_eq!(number(749.99999999), "750");
        assert_eq!(number(0.5), "0.5");
        assert_eq!(number(14.5), "14.5");
        assert_eq!(number(2.87e-21), "2.87000e-21");
        assert_eq!(with_unit(750.0, "K"), "750 K");
        assert_eq!(with_unit(0.6, "dimensionless"), "0.6");
    }
}
