//! Automatic checks of a definition, as a typed report: dimensions,
//! monotonicity per parameter, finiteness and non-negativity of the bound,
//! and the validity conditions, on a sampled range.

use limina_core::{
    Configuration, ConfigurationSet, ConstraintsMap, DomainRange, EngineError, Interval,
};

use crate::definition::{LimitDefinition, Status};
use crate::error::DslError;
use crate::ivops::{holds, Values};
use crate::limit::DeclarativeLimit;

/// Points per parameter for the monotonicity check.
pub const MONOTONICITY_SAMPLES: usize = 41;
/// Points per parameter on the grid for finiteness, sign and validity.
pub const GRID_SAMPLES: usize = 9;
/// Largest number of configurations reported.
const MAX_CONFIGURATIONS: usize = 256;

/// How the bound moves with one parameter, the others held at mid-range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Monotonicity {
    /// Never decreases, and increases somewhere.
    Increasing,
    /// Never increases, and decreases somewhere.
    Decreasing,
    /// Both increases and decreases.
    NonMonotone,
    /// No range to sample, too few valid points, or constant.
    Undetermined,
}

/// Result of the dimension check.
#[derive(Debug, Clone, PartialEq)]
pub enum DimensionCheck {
    /// Every formula and condition is consistent.
    Passed,
    /// The definition was refused.
    Failed(DslError),
}

/// Monotonicity in one parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterTrend {
    /// Parameter name.
    pub parameter: String,
    /// The trend.
    pub monotonicity: Monotonicity,
}

/// Checks of the bound under one configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaReport {
    /// The configuration, as "assumption = option" pairs.
    pub configuration: String,
    /// The formula used; None when the configuration is not applicable.
    pub formula: Option<String>,
    /// Monotonicity per parameter.
    pub trends: Vec<ParameterTrend>,
    /// Grid points where the bound was evaluated.
    pub samples: usize,
    /// Whether the bound was finite at every evaluated point; None without a grid.
    pub finite: Option<bool>,
    /// Whether the bound was non-negative at every evaluated point; None without a grid.
    pub non_negative: Option<bool>,
}

/// How often a validity condition held on the grid.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionReport {
    /// The condition.
    pub condition: String,
    /// Grid points where it held.
    pub held: usize,
    /// Grid points where it failed.
    pub failed: usize,
}

/// The whole report.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckReport {
    /// Limit id.
    pub limit_id: String,
    /// Status as written.
    pub status: Status,
    /// True only for `validated` with a person in `validated_by`.
    pub validated: bool,
    /// (a) Dimension check.
    pub dimension: DimensionCheck,
    /// (b) and (c), per configuration.
    pub formulas: Vec<FormulaReport>,
    /// (d) Validity conditions on the grid.
    pub validity: Vec<ConditionReport>,
}

/// Checks a definition; a refused one gets a report with the error.
pub fn check_definition(definition: &LimitDefinition) -> CheckReport {
    match DeclarativeLimit::compile(definition.clone()) {
        Ok(limit) => check_limit(&limit),
        Err(e) => CheckReport {
            limit_id: definition.id.clone(),
            status: definition.status,
            validated: definition.is_validated(),
            dimension: DimensionCheck::Failed(e),
            formulas: Vec::new(),
            validity: Vec::new(),
        },
    }
}

/// The sampled range of each parameter: its test range, else its plausible
/// domain. None for a parameter without either.
fn ranges(limit: &DeclarativeLimit) -> Vec<Option<(f64, f64)>> {
    limit
        .definition()
        .parameters
        .iter()
        .map(|p| {
            if let Some([lo, hi]) = p.test_range {
                return Some((lo, hi));
            }
            let spec = limina_core::Limit::parameters(limit)
                .iter()
                .find(|s| s.id.as_str() == p.name)?;
            match &spec.plausible.as_ref()?.range {
                DomainRange::Interval(i) => Some((i.lo(), i.hi())),
                DomainRange::Finite(v) => {
                    let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
                    let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    Some((lo, hi))
                }
            }
        })
        .collect()
}

pub(crate) fn samples(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    if n < 2 || lo == hi {
        return vec![lo];
    }
    (0..n)
        .map(|i| lo + (hi - lo) * i as f64 / (n - 1) as f64)
        .collect()
}

fn values_at(names: &[String], point: &[f64]) -> Option<Values> {
    names
        .iter()
        .zip(point)
        .map(|(n, v)| Interval::point(*v).ok().map(|i| (n.clone(), i)))
        .collect()
}

pub(crate) fn trend(series: &[f64]) -> Monotonicity {
    if series.len() < 2 {
        return Monotonicity::Undetermined;
    }
    let (mut up, mut down) = (false, false);
    for w in series.windows(2) {
        let tol = 1e-12 * w[0].abs().max(w[1].abs());
        if w[1] - w[0] > tol {
            up = true;
        } else if w[0] - w[1] > tol {
            down = true;
        }
    }
    match (up, down) {
        (true, false) => Monotonicity::Increasing,
        (false, true) => Monotonicity::Decreasing,
        (true, true) => Monotonicity::NonMonotone,
        (false, false) => Monotonicity::Undetermined,
    }
}

fn describe(config: &Configuration) -> String {
    let pairs: Vec<String> = config
        .iter()
        .map(|(a, o)| format!("{} = {}", a.as_str(), o.as_str()))
        .collect();
    if pairs.is_empty() {
        "(no assumption)".to_string()
    } else {
        pairs.join(", ")
    }
}

/// Every grid point: the cartesian product of the samples of each range.
fn grid(ranges: &[(f64, f64)]) -> Vec<Vec<f64>> {
    let mut points = vec![Vec::new()];
    for (lo, hi) in ranges {
        let s = samples(*lo, *hi, GRID_SAMPLES);
        points = points
            .into_iter()
            .flat_map(|p| s.iter().map(move |v| [p.clone(), vec![*v]].concat()))
            .collect();
    }
    points
}

/// Checks a compiled limit.
pub fn check_limit(limit: &DeclarativeLimit) -> CheckReport {
    let names = limit.parameter_names().to_vec();
    let ranges = ranges(limit);
    let all: Option<Vec<(f64, f64)>> = ranges.iter().copied().collect();
    let points = all.as_ref().map(|r| grid(r));
    let validity = limit
        .validity()
        .iter()
        .map(|c| {
            let (mut held, mut failed) = (0, 0);
            for p in points.iter().flatten() {
                match values_at(&names, p).map(|v| holds(c, &v)) {
                    Some(Ok(true)) => held += 1,
                    _ => failed += 1,
                }
            }
            ConditionReport {
                condition: c.text.clone(),
                held,
                failed,
            }
        })
        .collect();
    let configurations = ConfigurationSet::build(limit, &ConstraintsMap::new(), MAX_CONFIGURATIONS)
        .map(|s| s.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let formulas = configurations
        .iter()
        .map(|config| {
            let Ok(formula) = limit.formula_for(config) else {
                return FormulaReport {
                    configuration: describe(config),
                    formula: None,
                    trends: Vec::new(),
                    samples: 0,
                    finite: None,
                    non_negative: None,
                };
            };
            let at = |point: &[f64]| -> Option<Result<Interval, EngineError>> {
                let values = values_at(&names, point)?;
                match limit.evaluate(config, &formula.expr, &values) {
                    Err(
                        EngineError::InvalidTemperatureOrder { .. }
                        | EngineError::NotApplicable { .. },
                    ) => None,
                    other => Some(other),
                }
            };
            let trends = names
                .iter()
                .enumerate()
                .map(|(k, name)| {
                    let monotonicity = match (&all, ranges[k]) {
                        (Some(r), Some((lo, hi))) => {
                            let mid: Vec<f64> = r.iter().map(|(a, b)| (a + b) / 2.0).collect();
                            let series: Vec<f64> = samples(lo, hi, MONOTONICITY_SAMPLES)
                                .into_iter()
                                .filter_map(|v| {
                                    let mut p = mid.clone();
                                    p[k] = v;
                                    at(&p)?.ok().map(|i| i.lo())
                                })
                                .collect();
                            trend(&series)
                        }
                        _ => Monotonicity::Undetermined,
                    };
                    ParameterTrend {
                        parameter: name.clone(),
                        monotonicity,
                    }
                })
                .collect();
            let (mut count, mut finite, mut non_negative) = (0, true, true);
            let grid_points: Vec<Vec<f64>> = match &points {
                Some(p) => p.clone(),
                None if names.is_empty() => vec![Vec::new()],
                None => Vec::new(),
            };
            for p in &grid_points {
                match at(p) {
                    Some(Ok(i)) => {
                        count += 1;
                        non_negative &= i.lo() >= 0.0;
                    }
                    Some(Err(_)) => {
                        count += 1;
                        finite = false;
                    }
                    None => {}
                }
            }
            let has_grid = !grid_points.is_empty();
            FormulaReport {
                configuration: describe(config),
                formula: Some(formula.text.clone()),
                trends,
                samples: count,
                finite: has_grid.then_some(finite),
                non_negative: has_grid.then_some(non_negative),
            }
        })
        .collect();
    CheckReport {
        limit_id: limit.definition().id.clone(),
        status: limit.definition().status,
        validated: limit.is_validated(),
        dimension: DimensionCheck::Passed,
        formulas,
        validity,
    }
}
