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
    let set = ConfigurationSet::build(limit, constraints, 1024)?;
    let mut base = BTreeMap::new();
    for (name, v) in fixed {
        if name != free {
            base.insert(ParameterId::new(name.clone()), Interval::point(*v)?);
        }
    }
    let mut out = Vec::with_capacity(points);
    for i in 0..points {
        let x = lo + (hi - lo) * i as f64 / (points - 1) as f64;
        let mut map = base.clone();
        map.insert(ParameterId::new(free), Interval::point(x)?);
        let enclosure = ParameterEnclosure::new(map);
        let mut range: Option<(f64, f64)> = None;
        let mut defined = true;
        for config in set.iter() {
            match limit.bound(config, &enclosure) {
                Ok(b) => {
                    range = Some(match range {
                        None => (b.lo(), b.hi()),
                        Some((l, h)) => (l.min(b.lo()), h.max(b.hi())),
                    })
                }
                Err(_) => defined = false,
            }
        }
        let (b_min, b_max) = match (defined, range) {
            (true, Some((l, h))) => (Some(l), Some(h)),
            _ => (None, None),
        };
        out.push(CurvePoint { x, b_min, b_max });
    }
    Ok(out)
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
