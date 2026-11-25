//! The bound as a function of one parameter, every other one fixed,
//! evaluated by the limit itself on point enclosures. Deterministic, for
//! plotting; it computes no verdict.

use std::collections::BTreeMap;

use limina_core::{
    ConfigurationSet, ConstraintsMap, Direction, EngineError, Interval, Limit, ParameterEnclosure,
    ParameterId,
};

/// Largest number of points of one curve.
pub const MAX_CURVE_POINTS: usize = 1000;

/// One point: the smallest and largest bound over the configurations, or
/// None where the bound is undefined (a validity condition fails).
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePoint {
    /// Value of the free parameter, SI.
    pub x: f64,
    /// Smallest bound over the configurations.
    pub b_min: Option<f64>,
    /// Largest bound over the configurations.
    pub b_max: Option<f64>,
}

/// Why no curve is given.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CurveError {
    /// The free parameter is not declared by the limit.
    #[error("the limit has no parameter {0}")]
    UnknownFreeParameter(String),
    /// A required parameter other than the free one has no value.
    #[error("parameter {0} needs a fixed value")]
    MissingFixedParameter(String),
    /// The range is not finite or not increasing, or the point count is
    /// outside 2 to 1000.
    #[error("invalid curve request: {0}")]
    Invalid(String),
    /// The engine refused the input.
    #[error(transparent)]
    Engine(#[from] EngineError),
}

/// `points` evenly spaced values of `free` over `range`, the other
/// parameters at `fixed` (SI values), each with the bound range over the
/// configurations allowed by `constraints`.
pub fn bound_curve(
    limit: &dyn Limit,
    constraints: &ConstraintsMap,
    fixed: &BTreeMap<String, f64>,
    free: &str,
    range: (f64, f64),
    points: usize,
) -> Result<Vec<CurvePoint>, CurveError> {
    let enclosed: BTreeMap<String, (f64, f64)> =
        fixed.iter().map(|(k, v)| (k.clone(), (*v, *v))).collect();
    bound_curve_enclosed(limit, constraints, &enclosed, free, range, points)
}

/// Like [`bound_curve`], each fixed parameter given as an SI enclosure
/// `(lo, hi)`: the bound range then includes the effect of the parameter
/// uncertainties, on top of the uncertainty of the constants, which the
/// limit already carries. This gives the uncertainty band of a curve.
pub fn bound_curve_enclosed(
    limit: &dyn Limit,
    constraints: &ConstraintsMap,
    fixed: &BTreeMap<String, (f64, f64)>,
    free: &str,
    range: (f64, f64),
    points: usize,
) -> Result<Vec<CurvePoint>, CurveError> {
    if !limit.parameters().iter().any(|p| p.id.as_str() == free) {
        return Err(CurveError::UnknownFreeParameter(free.to_string()));
    }
    for p in limit.parameters() {
        if p.id.as_str() != free && p.required && !fixed.contains_key(p.id.as_str()) {
            return Err(CurveError::MissingFixedParameter(p.id.as_str().to_string()));
        }
    }
    let (lo, hi) = range;
    if !(lo.is_finite() && hi.is_finite() && lo < hi) {
        return Err(CurveError::Invalid(format!("range [{lo}, {hi}]")));
    }
    if !(2..=MAX_CURVE_POINTS).contains(&points) {
        return Err(CurveError::Invalid(format!("{points} points")));
    }
    let xs = sample_points(range, points, false);
    bound_curve_at(limit, constraints, fixed, free, &xs)
}

/// Smallest ratio `hi / lo` for which [`sample_points`] in automatic mode
/// spaces the samples logarithmically: three decades.
pub const LOG_SPACING_RATIO: f64 = 1e3;

/// Whether a range spans enough decades, with `lo > 0`, for logarithmic
/// sampling and a logarithmic axis.
pub fn wide_range(range: (f64, f64)) -> bool {
    range.0 > 0.0 && range.1 / range.0 >= LOG_SPACING_RATIO
}

/// `points` values from `range.0` to `range.1`: evenly spaced, or evenly
/// spaced in logarithm when `log` is true and `range.0 > 0`. The two ends
/// are exact.
pub fn sample_points(range: (f64, f64), points: usize, log: bool) -> Vec<f64> {
    let (lo, hi) = range;
    let n = points.max(2);
    let logarithmic = log && lo > 0.0;
    (0..n)
        .map(|i| {
            if !logarithmic {
                // The same arithmetic as before D-25 phase 5, so linear
                // curves are unchanged to the last bit.
                return lo + (hi - lo) * i as f64 / (n - 1) as f64;
            }
            if i == 0 {
                return lo;
            }
            if i == n - 1 {
                return hi;
            }
            let t = i as f64 / (n - 1) as f64;
            (lo.ln() + t * (hi.ln() - lo.ln())).exp()
        })
        .collect()
}

/// Like [`bound_curve_enclosed`], at the given values of the free
/// parameter (finite, at most [`MAX_CURVE_POINTS`]).
pub fn bound_curve_at(
    limit: &dyn Limit,
    constraints: &ConstraintsMap,
    fixed: &BTreeMap<String, (f64, f64)>,
    free: &str,
    xs: &[f64],
) -> Result<Vec<CurvePoint>, CurveError> {
    if !limit.parameters().iter().any(|p| p.id.as_str() == free) {
        return Err(CurveError::UnknownFreeParameter(free.to_string()));
    }
    for p in limit.parameters() {
        if p.id.as_str() != free && p.required && !fixed.contains_key(p.id.as_str()) {
            return Err(CurveError::MissingFixedParameter(p.id.as_str().to_string()));
        }
    }
    if xs.len() < 2 || xs.len() > MAX_CURVE_POINTS || xs.iter().any(|x| !x.is_finite()) {
        return Err(CurveError::Invalid(format!("{} points", xs.len())));
    }
    let set = ConfigurationSet::build(limit, constraints, 1024)?;
    let mut base = BTreeMap::new();
    for (name, v) in fixed {
        if name != free {
            base.insert(ParameterId::new(name.clone()), Interval::new(v.0, v.1)?);
        }
    }
    let mut out = Vec::with_capacity(xs.len());
    for &x in xs {
        let mut map = base.clone();
        map.insert(ParameterId::new(free), Interval::point(x)?);
        let (b_min, b_max) = bound_range(limit, &set, &ParameterEnclosure::new(map));
        out.push(CurvePoint { x, b_min, b_max });
    }
    Ok(out)
}

/// Smallest and largest bound over the configurations at one enclosure;
/// None where the bound is undefined in any configuration.
pub(crate) fn bound_range(
    limit: &dyn Limit,
    set: &ConfigurationSet,
    enclosure: &ParameterEnclosure,
) -> (Option<f64>, Option<f64>) {
    let mut range: Option<(f64, f64)> = None;
    let mut defined = true;
    for config in set.iter() {
        match limit.bound(config, enclosure) {
            Ok(b) => {
                range = Some(match range {
                    None => (b.lo(), b.hi()),
                    Some((l, h)) => (l.min(b.lo()), h.max(b.hi())),
                })
            }
            Err(_) => defined = false,
        }
    }
    match (defined, range) {
        (true, Some((l, h))) => (Some(l), Some(h)),
        _ => (None, None),
    }
}

/// Intervals of `x` where `outside` holds, from the first to the last
/// sample of each run of consecutive samples.
pub fn zones<F: Fn(&CurvePoint) -> bool>(points: &[CurvePoint], outside: F) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    let mut open: Option<f64> = None;
    let mut last = f64::NAN;
    for p in points {
        match (outside(p), open) {
            (true, None) => open = Some(p.x),
            (false, Some(start)) => {
                out.push((start, last));
                open = None;
            }
            _ => {}
        }
        last = p.x;
    }
    if let Some(start) = open {
        out.push((start, last));
    }
    out
}

/// Validity zones of a curve: where the bound is undefined because a
/// validity condition of the law fails.
pub fn validity_zones(points: &[CurvePoint]) -> Vec<(f64, f64)> {
    zones(points, |p| p.b_max.is_none())
}

/// The bound that decides IMPOSSIBLE at a point: a claim above `b_max`
/// (Max limit) or below `b_min` (Min limit) is inconsistent in every
/// configuration.
pub fn decisive(direction: Direction, point: &CurvePoint) -> Option<f64> {
    match direction {
        Direction::Max => point.b_max,
        Direction::Min => point.b_min,
    }
}
