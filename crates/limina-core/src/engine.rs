//! The decision engine (design 6). Pure functions: no I/O, no hidden state.

use crate::config::ConfigurationSet;
use crate::error::EngineError;
use crate::interval::Interval;
use crate::limit::{Configuration, Direction, Limit, ParameterEnclosure};
use crate::verdict::BoundRange;

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
