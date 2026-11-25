//! The bound as a function of two parameters, every other one fixed,
//! evaluated by the limit itself. Deterministic, for the heat map of the
//! explorer; it computes no verdict.
use std::collections::BTreeMap;

use limina_core::{
    ConfigurationSet, ConstraintsMap, Direction, Interval, Limit, ParameterEnclosure, ParameterId,
};

use crate::curve::{bound_range, CurveError};

/// Largest number of samples along one axis of a surface.
pub const MAX_SURFACE_AXIS: usize = 50;

/// One free axis of a surface: parameter, SI range, number of samples.
#[derive(Debug, Clone)]
pub struct SurfaceAxis {
    /// Parameter id.
    pub name: String,
    /// SI range, increasing.
    pub range: (f64, f64),
    /// Number of samples, 2 to [`MAX_SURFACE_AXIS`].
    pub samples: usize,
}

impl SurfaceAxis {
    fn check(&self, limit: &dyn Limit) -> Result<(), CurveError> {
        if !limit
            .parameters()
            .iter()
            .any(|p| p.id.as_str() == self.name)
        {
            return Err(CurveError::UnknownFreeParameter(self.name.clone()));
        }
        let (lo, hi) = self.range;
        if !(lo.is_finite() && hi.is_finite() && lo < hi) {
            return Err(CurveError::Invalid(format!("range [{lo}, {hi}]")));
        }
        if !(2..=MAX_SURFACE_AXIS).contains(&self.samples) {
            return Err(CurveError::Invalid(format!("{} samples", self.samples)));
        }
        Ok(())
    }

    /// The sample values, evenly spaced.
    pub fn values(&self) -> Vec<f64> {
        let (lo, hi) = self.range;
        (0..self.samples)
            .map(|i| lo + (hi - lo) * i as f64 / (self.samples - 1) as f64)
            .collect()
    }
}

/// The deciding bound (largest for a Max limit, smallest for a Min limit)
/// on the grid `y` by `x`: `rows[j][i]` is at `y[j]`, `x[i]`; None where the
/// bound is undefined.
pub fn bound_surface(
    limit: &dyn Limit,
    constraints: &ConstraintsMap,
    fixed: &BTreeMap<String, f64>,
    x: &SurfaceAxis,
    y: &SurfaceAxis,
) -> Result<Vec<Vec<Option<f64>>>, CurveError> {
    x.check(limit)?;
    y.check(limit)?;
    if x.name == y.name {
        return Err(CurveError::Invalid(
            "the two axes are the same parameter".into(),
        ));
    }
    for p in limit.parameters() {
        let name = p.id.as_str();
        if name != x.name && name != y.name && p.required && !fixed.contains_key(name) {
            return Err(CurveError::MissingFixedParameter(name.to_string()));
        }
    }
    let set = ConfigurationSet::build(limit, constraints, 1024)?;
    let mut base = BTreeMap::new();
    for (name, v) in fixed {
        if name != &x.name && name != &y.name {
            base.insert(ParameterId::new(name.clone()), Interval::point(*v)?);
        }
    }
    let (xs, ys) = (x.values(), y.values());
    let mut rows = Vec::with_capacity(ys.len());
    for yv in &ys {
        let mut row = Vec::with_capacity(xs.len());
        for xv in &xs {
            let mut map = base.clone();
            map.insert(ParameterId::new(x.name.clone()), Interval::point(*xv)?);
            map.insert(ParameterId::new(y.name.clone()), Interval::point(*yv)?);
            let (b_min, b_max) = bound_range(limit, &set, &ParameterEnclosure::new(map));
            row.push(match limit.direction() {
                Direction::Max => b_max,
                Direction::Min => b_min,
            });
        }
        rows.push(row);
    }
    Ok(rows)
}
