//! Numerical comparison of two definitions of the same law, written
//! independently (for example by two language models).
//!
//! Two definitions agree when they declare the same direction, bounded
//! quantity and parameters, and their decisive bounds match within a
//! relative tolerance at every sampled point inside the test ranges. The
//! decisive bound is the one that decides IMPOSSIBLE: the largest bound over
//! the configurations for a Max limit, the smallest for a Min limit, so two
//! writers who split the same law into assumptions differently can still
//! agree. The sampling is deterministic (fixed seed). Agreement is evidence
//! that two writers read the law the same way; it is not a validation.

use std::collections::BTreeMap;
use std::mem::discriminant;

use limina_core::{
    ConfigurationSet, ConstraintsMap, Direction, DomainRange, EngineError, Interval, Limit,
    MeasureKind, ParameterEnclosure, ParameterId,
};

use crate::limit::DeclarativeLimit;

/// Samples drawn by [`compare_limits`] in the default comparison.
pub const COMPARISON_SAMPLES: usize = 200;
/// Largest relative difference for agreement.
pub const AGREEMENT_TOLERANCE: f64 = 1e-6;
/// Fixed seed of the sampling.
pub const COMPARISON_SEED: u64 = 0x4C49_4D49_4E41;

/// The result of a comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    /// True only when every check and every sample agrees.
    pub agreed: bool,
    /// Samples evaluated.
    pub samples: usize,
    /// Largest relative difference seen over the samples (0 without samples).
    pub max_relative_difference: f64,
    /// Why they do not agree; None when they agree.
    pub reason: Option<String>,
}

fn disagree(reason: String, samples: usize, max: f64) -> Comparison {
    Comparison {
        agreed: false,
        samples,
        max_relative_difference: max,
        reason: Some(reason),
    }
}

/// SplitMix64: a small deterministic generator, so no dependency is needed.
struct Rng(u64);

impl Rng {
    fn next_unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The sampling range of a parameter: its plausible interval, else its
/// test range.
fn range_of(limit: &DeclarativeLimit, name: &str) -> Option<(f64, f64)> {
    let spec = limit.parameters().iter().find(|p| p.id.as_str() == name)?;
    if let Some(domain) = &spec.plausible {
        if let DomainRange::Interval(i) = &domain.range {
            return Some((i.lo(), i.hi()));
        }
    }
    limit
        .definition()
        .parameters
        .iter()
        .find(|p| p.name == name)
        .and_then(|p| p.test_range)
        .map(|[lo, hi]| (lo, hi))
}

fn sample(rng: &mut Rng, (lo, hi): (f64, f64)) -> f64 {
    let t = rng.next_unit();
    if lo > 0.0 && hi / lo > 100.0 {
        (lo.ln() + t * (hi.ln() - lo.ln())).exp()
    } else {
        lo + t * (hi - lo)
    }
}

/// `|a - b| / max(|a|, |b|)`, and 0 when both are equal.
pub fn relative_difference(a: f64, b: f64) -> f64 {
    if a == b {
        0.0
    } else {
        (a - b).abs() / a.abs().max(b.abs())
    }
}

fn kind_name(kind: MeasureKind) -> String {
    match kind {
        MeasureKind::AbsoluteTemperature => "absolute temperature".to_string(),
        MeasureKind::TemperatureDifference => "temperature difference".to_string(),
        MeasureKind::Plain(d) => format!("{:?}", d.exponents()),
    }
}

fn parameters(limit: &DeclarativeLimit) -> BTreeMap<String, String> {
    limit
        .parameters()
        .iter()
        .map(|p| (p.id.as_str().to_string(), kind_name(p.kind)))
        .collect()
}

/// The structural checks; the reason of the first difference.
fn same_shape(a: &DeclarativeLimit, b: &DeclarativeLimit) -> Option<String> {
    if a.direction() != b.direction() {
        return Some("the two definitions bound the quantity in opposite directions".into());
    }
    if a.bounded_kind() != b.bounded_kind() {
        return Some(format!(
            "bounded quantity differs: {} against {}",
            kind_name(a.bounded_kind()),
            kind_name(b.bounded_kind())
        ));
    }
    let (pa, pb) = (parameters(a), parameters(b));
    if pa != pb {
        return Some(format!(
            "parameters differ: {:?} against {:?}",
            pa.keys().collect::<Vec<_>>(),
            pb.keys().collect::<Vec<_>>()
        ));
    }
    None
}

/// The decisive bound of `limit` at a point: largest upper end over the
/// configurations (Max) or smallest lower end (Min). An error when any
/// configuration is undefined there.
fn decisive_bound(
    limit: &DeclarativeLimit,
    set: &ConfigurationSet,
    enclosure: &ParameterEnclosure,
) -> Result<f64, EngineError> {
    let mut out: Option<f64> = None;
    for config in set.iter() {
        let b = limit.bound(config, enclosure)?;
        out = Some(match (limit.direction(), out) {
            (Direction::Max, None) => b.hi(),
            (Direction::Max, Some(o)) => o.max(b.hi()),
            (Direction::Min, None) => b.lo(),
            (Direction::Min, Some(o)) => o.min(b.lo()),
        });
    }
    out.ok_or(EngineError::EmptyConfigurationSet)
}

/// The decisive bound of `limit` (largest upper end for a Max law, smallest
/// lower end for a Min law, over every configuration) at the given SI
/// parameter values, computed by the engine as in [`compare_limits`]. An
/// error names the first parameter missing or the engine error.
pub fn decisive_bound_at(
    limit: &DeclarativeLimit,
    values: &BTreeMap<String, f64>,
) -> Result<f64, String> {
    let mut map = BTreeMap::new();
    for p in limit.parameters() {
        let name = p.id.as_str();
        let Some(v) = values.get(name) else {
            return Err(format!("parameter {name} not read from the text"));
        };
        let iv = Interval::point(*v).map_err(|e| format!("{name} = {v}: {e}"))?;
        map.insert(ParameterId::new(name.to_string()), iv);
    }
    let set =
        ConfigurationSet::build(limit, &ConstraintsMap::new(), 1024).map_err(|e| e.to_string())?;
    decisive_bound(limit, &set, &ParameterEnclosure::new(map)).map_err(|e| e.to_string())
}

/// Compares `a` and `b` on `samples` points drawn with `seed`, each
/// parameter inside the intersection of the two ranges.
pub fn compare_limits(
    a: &DeclarativeLimit,
    b: &DeclarativeLimit,
    samples: usize,
    seed: u64,
) -> Comparison {
    if let Some(reason) = same_shape(a, b) {
        return disagree(reason, 0, 0.0);
    }
    let mut ranges = Vec::new();
    for name in parameters(a).keys() {
        let (Some((alo, ahi)), Some((blo, bhi))) = (range_of(a, name), range_of(b, name)) else {
            return disagree(format!("no test range for {name}"), 0, 0.0);
        };
        let (lo, hi) = (alo.max(blo), ahi.min(bhi));
        if !(lo.is_finite() && hi.is_finite() && lo <= hi) {
            return disagree(format!("the test ranges of {name} do not overlap"), 0, 0.0);
        }
        ranges.push((name.clone(), (lo, hi)));
    }
    let sets = (
        ConfigurationSet::build(a, &ConstraintsMap::new(), 1024),
        ConfigurationSet::build(b, &ConstraintsMap::new(), 1024),
    );
    let (set_a, set_b) = match sets {
        (Ok(x), Ok(y)) => (x, y),
        (Err(e), _) | (_, Err(e)) => return disagree(format!("configuration set: {e}"), 0, 0.0),
    };
    let mut rng = Rng(seed);
    let mut max = 0.0f64;
    for i in 0..samples {
        let mut map = BTreeMap::new();
        let mut point = Vec::new();
        for (name, range) in &ranges {
            let v = sample(&mut rng, *range);
            point.push(format!("{name} = {v:e}"));
            match Interval::point(v) {
                Ok(iv) => {
                    map.insert(ParameterId::new(name.clone()), iv);
                }
                Err(e) => return disagree(format!("sample {v}: {e}"), i, max),
            }
        }
        let enclosure = ParameterEnclosure::new(map);
        let x = decisive_bound(a, &set_a, &enclosure);
        let y = decisive_bound(b, &set_b, &enclosure);
        match (&x, &y) {
            (Ok(x), Ok(y)) => {
                let d = relative_difference(*x, *y);
                max = max.max(d);
                if d.is_nan() || d > AGREEMENT_TOLERANCE {
                    return disagree(
                        format!(
                            "bounds differ at {}: {x:e} against {y:e} (relative difference {d:e})",
                            point.join(", ")
                        ),
                        i + 1,
                        max,
                    );
                }
            }
            (Err(x), Err(y)) if discriminant(x) == discriminant(y) => {}
            _ => {
                let show = |r: &Result<f64, EngineError>| match r {
                    Ok(v) => format!("{v:e}"),
                    Err(e) => e.to_string(),
                };
                return disagree(
                    format!(
                        "at {} one definition gives {} and the other {}",
                        point.join(", "),
                        show(&x),
                        show(&y)
                    ),
                    i + 1,
                    max,
                );
            }
        }
    }
    Comparison {
        agreed: true,
        samples,
        max_relative_difference: max,
        reason: None,
    }
}
