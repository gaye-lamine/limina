//! The `Limit` trait and the types a limit declares (design 5).
//!
//! This module defines no physical limit. Limits are implemented in their own
//! files, each from its own spec.

use std::collections::BTreeMap;

use crate::error::EngineError;
use crate::interval::Interval;
use crate::quantity::{Measure, MeasureKind};

macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// Builds the id from any string.
            pub fn new(s: impl Into<String>) -> Self {
                $name(s.into())
            }

            /// The id as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id!(
    /// Identifier of an assumption of a limit.
    AssumptionId
);
string_id!(
    /// Identifier of one option in the domain of an assumption.
    OptionId
);
string_id!(
    /// Identifier of a parameter `p` of a bound function.
    ParameterId
);

/// Direction of a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The claim must not exceed the bound.
    Max,
    /// The claim must be at least the bound.
    Min,
}

/// One allowed value of an assumption. Equality is on `id` only.
#[derive(Debug, Clone)]
pub struct AssumptionOption {
    /// Identifier of the option.
    pub id: OptionId,
    /// Optional value attached to the option.
    pub value: Option<Measure>,
}

impl PartialEq for AssumptionOption {
    fn eq(&self, other: &AssumptionOption) -> bool {
        self.id == other.id
    }
}

/// A named assumption with a finite declared domain.
#[derive(Debug, Clone)]
pub struct Assumption {
    /// Identifier of the assumption.
    pub id: AssumptionId,
    /// Declared domain: the allowed options.
    pub domain: Vec<AssumptionOption>,
}

/// Plausible range of a parameter, in SI base units, in the kind of the
/// parameter. It is never enumerated: the engine uses only its endpoints
/// (R-08.7).
#[derive(Debug, Clone)]
pub enum DomainRange {
    /// A closed interval.
    Interval(Interval),
    /// A finite set of values, used through its hull `[min, max]`.
    Finite(Vec<f64>),
}

/// Plausible domain of a parameter, with its source (D-04).
#[derive(Debug, Clone)]
pub struct PlausibleDomain {
    /// The range of plausible values.
    pub range: DomainRange,
    /// Where the range comes from, as written in the limit's spec.
    pub source: String,
}

/// A parameter `p` of `B`. A `required` parameter that is not supplied is
/// "missing".
#[derive(Debug, Clone)]
pub struct ParameterSpec {
    /// Identifier of the parameter.
    pub id: ParameterId,
    /// Kind of the parameter value.
    pub kind: MeasureKind,
    /// Whether the bound needs this parameter.
    pub required: bool,
    /// Declared plausible domain; `None` means no declared domain (R-05.5).
    pub plausible: Option<PlausibleDomain>,
}

/// Values of the parameters during one evaluation, in SI base units: a point
/// interval for a supplied parameter, the hull of the plausible domain for a
/// missing one.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParameterEnclosure(BTreeMap<ParameterId, Interval>);

impl ParameterEnclosure {
    /// Builds an enclosure from parameter values.
    pub fn new(values: BTreeMap<ParameterId, Interval>) -> ParameterEnclosure {
        ParameterEnclosure(values)
    }

    /// Value of parameter `id`, if present.
    pub fn get(&self, id: &ParameterId) -> Option<Interval> {
        self.0.get(id).copied()
    }
}

/// One assignment of an option to every assumption of a limit.
/// Ordered, so configuration sets have a deterministic order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Configuration(BTreeMap<AssumptionId, OptionId>);

impl Configuration {
    /// Builds a configuration. Only `ConfigurationSet::build` creates
    /// configurations, so every configuration belongs to some `S`.
    pub(crate) fn from_map(options: BTreeMap<AssumptionId, OptionId>) -> Configuration {
        Configuration(options)
    }

    /// Option chosen for `assumption`, if the assumption is part of this
    /// configuration.
    pub fn option(&self, assumption: &AssumptionId) -> Option<&OptionId> {
        self.0.get(assumption)
    }

    /// Assumptions and their options, in assumption id order.
    pub fn iter(&self) -> impl Iterator<Item = (&AssumptionId, &OptionId)> {
        self.0.iter()
    }
}

/// A physical limit, as seen by the decision engine.
///
/// Every implementation declares its formula, assumptions and references in
/// its own spec (see `physics-rules.md`). The engine itself knows no physics.
pub trait Limit {
    /// Stable identifier of the limit.
    fn id(&self) -> &str;

    /// Whether the bound is a maximum or a minimum.
    fn direction(&self) -> Direction;

    /// Kind of the bounded quantity; the claim must have the same kind.
    fn bounded_kind(&self) -> MeasureKind;

    /// Declared assumptions, each with a finite domain.
    fn assumptions(&self) -> &[Assumption];

    /// Declared parameters of the bound function.
    fn parameters(&self) -> &[ParameterSpec];

    /// Range of `B(c, p)` in SI base units when each parameter ranges over
    /// `params`. For a point enclosure the result is a point interval. The
    /// limit builds it with `Interval` arithmetic. Errors propagate to the
    /// caller.
    fn bound(
        &self,
        config: &Configuration,
        params: &ParameterEnclosure,
    ) -> Result<Interval, EngineError>;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        Assumption, AssumptionId, AssumptionOption, Configuration, Direction, Limit, OptionId,
        ParameterEnclosure, ParameterId, ParameterSpec,
    };
    use crate::dimension::Dimension;
    use crate::error::EngineError;
    use crate::interval::Interval;
    use crate::quantity::MeasureKind;

    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::new(lo, hi).expect("valid interval")
    }

    fn config(pairs: &[(&str, &str)]) -> Configuration {
        Configuration(
            pairs
                .iter()
                .map(|(a, o)| (AssumptionId::new(*a), OptionId::new(*o)))
                .collect(),
        )
    }

    /// Fake limit for this test only: B = base(option of "mode") * p, no physics.
    struct FakeLimit {
        assumptions: Vec<Assumption>,
        parameters: Vec<ParameterSpec>,
    }

    impl FakeLimit {
        fn new() -> FakeLimit {
            FakeLimit {
                assumptions: vec![Assumption {
                    id: AssumptionId::new("mode"),
                    domain: vec![
                        AssumptionOption {
                            id: OptionId::new("low"),
                            value: None,
                        },
                        AssumptionOption {
                            id: OptionId::new("high"),
                            value: None,
                        },
                    ],
                }],
                parameters: vec![ParameterSpec {
                    id: ParameterId::new("p"),
                    kind: MeasureKind::Plain(Dimension::DIMENSIONLESS),
                    required: true,
                    plausible: None,
                }],
            }
        }
    }

    impl Limit for FakeLimit {
        fn id(&self) -> &str {
            "fake"
        }
        fn direction(&self) -> Direction {
            Direction::Max
        }
        fn bounded_kind(&self) -> MeasureKind {
            MeasureKind::Plain(Dimension::DIMENSIONLESS)
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
                .map(OptionId::as_str)
            {
                Some("low") => Interval::point(5.0)?,
                Some("high") => Interval::point(8.0)?,
                _ => return Err(EngineError::UnknownAssumption("mode".to_string())),
            };
            let p = params
                .get(&ParameterId::new("p"))
                .ok_or_else(|| EngineError::UnknownParameter("p".to_string()))?;
            base.mul(p)
        }
    }

    #[test]
    fn ids_compare_and_order_by_their_string() {
        assert_eq!(AssumptionId::new("a"), AssumptionId::new("a".to_string()));
        assert!(OptionId::new("a") < OptionId::new("b"));
        assert!(ParameterId::new("t_cold") < ParameterId::new("t_hot"));
        assert_eq!(ParameterId::new("p").as_str(), "p");
    }

    #[test]
    fn assumption_option_equality_is_on_id_only() {
        let a = AssumptionOption {
            id: OptionId::new("x"),
            value: None,
        };
        let b = AssumptionOption {
            id: OptionId::new("x"),
            value: Some(crate::quantity::Measure::Plain(
                crate::quantity::Quantity::from_unit(1.0, "m").expect("valid"),
            )),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn parameter_enclosure_get() {
        let mut values = BTreeMap::new();
        values.insert(ParameterId::new("p"), iv(1.0, 2.0));
        let params = ParameterEnclosure::new(values);
        assert_eq!(params.get(&ParameterId::new("p")), Some(iv(1.0, 2.0)));
        assert_eq!(params.get(&ParameterId::new("q")), None);
        assert_eq!(
            ParameterEnclosure::default().get(&ParameterId::new("p")),
            None
        );
    }

    #[test]
    fn configuration_option_and_iteration_order() {
        let c = config(&[("b", "y"), ("a", "x")]);
        assert_eq!(c.option(&AssumptionId::new("a")), Some(&OptionId::new("x")));
        assert_eq!(c.option(&AssumptionId::new("z")), None);
        let keys: Vec<&str> = c.iter().map(|(a, _)| a.as_str()).collect();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn bound_on_a_point_enclosure_is_a_point() {
        let limit: &dyn Limit = &FakeLimit::new();
        let mut values = BTreeMap::new();
        values.insert(ParameterId::new("p"), iv(2.0, 2.0));
        let params = ParameterEnclosure::new(values);
        assert_eq!(
            limit.bound(&config(&[("mode", "low")]), &params),
            Ok(iv(10.0, 10.0))
        );
        assert_eq!(
            limit.bound(&config(&[("mode", "high")]), &params),
            Ok(iv(16.0, 16.0))
        );
    }

    #[test]
    fn bound_on_a_range_enclosure_is_a_range() {
        let limit: &dyn Limit = &FakeLimit::new();
        let mut values = BTreeMap::new();
        values.insert(ParameterId::new("p"), iv(1.0, 2.0));
        let params = ParameterEnclosure::new(values);
        assert_eq!(
            limit.bound(&config(&[("mode", "high")]), &params),
            Ok(iv(8.0, 16.0))
        );
    }

    #[test]
    fn limit_errors_propagate() {
        let limit: &dyn Limit = &FakeLimit::new();
        assert_eq!(
            limit.bound(&config(&[("mode", "low")]), &ParameterEnclosure::default()),
            Err(EngineError::UnknownParameter("p".to_string()))
        );
    }
}
