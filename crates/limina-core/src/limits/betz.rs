//! The Betz limit on the power coefficient of a wind turbine.

use crate::dimension::Dimension;
use crate::error::EngineError;
use crate::interval::Interval;
use crate::limit::{
    Assumption, AssumptionId, AssumptionOption, Configuration, Direction, Limit, OptionId,
    ParameterEnclosure, ParameterSpec,
};
use crate::quantity::MeasureKind;

const LIMIT_ID: &str = "betz";
const REFERENCE_AREA: &str = "reference_area";
const UNSHROUDED_ROTOR: &str = "unshrouded_rotor";
const SHROUDED_TOTAL_CAPTURE_AREA: &str = "shrouded_total_capture_area";
const SHROUDED_ROTOR_AREA_ONLY: &str = "shrouded_rotor_area_only";

/// The Betz limit on the power coefficient `Cp` of a wind turbine.
///
/// Formula: `Cp <= 16/27` (about 0.593), from the actuator disk model.
/// `Cp` is dimensionless and the limit is a Max bound.
///
/// Epistemic status: model bound. It holds under the model assumptions:
/// incompressible, unconfined flow through an ideal actuator disk.
///
/// Assumption `reference_area`, the area `Cp` is referred to, with three
/// options:
/// - `unshrouded_rotor`: bound 16/27.
/// - `shrouded_total_capture_area`: bound 16/27.
/// - `shrouded_rotor_area_only`: outside the validity domain. A claim for a
///   shrouded turbine referred to the rotor area alone can exceed 16/27, so
///   the limit computes no bound and `bound` returns
///   `EngineError::NotApplicable`.
///
/// Callers must constrain `reference_area`. An unconstrained claim keeps all
/// three options, so `decide` returns `NotApplicable` and no verdict. A later
/// extraction step will map `NotApplicable` to INSUFFICIENT_DATA
/// (reference area not stated).
///
/// A claim above the bound is reported as "inconsistent with limit betz under
/// assumptions reference_area = ...".
///
/// Reference: // TODO(reference): to be supplied by the author
// TODO(reference): to be supplied by the author
#[derive(Debug, Clone)]
pub struct Betz {
    assumptions: Vec<Assumption>,
    parameters: Vec<ParameterSpec>,
}

impl Betz {
    /// Builds the limit with its `reference_area` assumption.
    pub fn new() -> Betz {
        let domain = [
            UNSHROUDED_ROTOR,
            SHROUDED_TOTAL_CAPTURE_AREA,
            SHROUDED_ROTOR_AREA_ONLY,
        ]
        .iter()
        .map(|id| AssumptionOption {
            id: OptionId::new(*id),
            value: None,
        })
        .collect();
        Betz {
            assumptions: vec![Assumption {
                id: AssumptionId::new(REFERENCE_AREA),
                domain,
            }],
            parameters: Vec::new(),
        }
    }
}

impl Default for Betz {
    fn default() -> Betz {
        Betz::new()
    }
}

impl Limit for Betz {
    fn id(&self) -> &str {
        LIMIT_ID
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
        _params: &ParameterEnclosure,
    ) -> Result<Interval, EngineError> {
        let assumption = AssumptionId::new(REFERENCE_AREA);
        let option = config
            .option(&assumption)
            .ok_or_else(|| EngineError::UnknownAssumption(REFERENCE_AREA.to_string()))?;
        match option.as_str() {
            UNSHROUDED_ROTOR | SHROUDED_TOTAL_CAPTURE_AREA => Interval::point(16.0 / 27.0),
            SHROUDED_ROTOR_AREA_ONLY => Err(EngineError::NotApplicable {
                limit_id: LIMIT_ID.to_string(),
                assumption: REFERENCE_AREA.to_string(),
                option: SHROUDED_ROTOR_AREA_ONLY.to_string(),
            }),
            other => Err(EngineError::ConstraintOutsideDomain {
                assumption: REFERENCE_AREA.to_string(),
                option: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::tolerance::approx_eq;

    fn config(option: &str) -> Configuration {
        let mut map = BTreeMap::new();
        map.insert(AssumptionId::new(REFERENCE_AREA), OptionId::new(option));
        Configuration::from_map(map)
    }

    fn bound_of(option: &str) -> Result<Interval, EngineError> {
        Betz::new().bound(&config(option), &ParameterEnclosure::default())
    }

    #[test]
    fn declarations_match_the_spec() {
        let limit = Betz::new();
        assert_eq!(limit.id(), "betz");
        assert_eq!(limit.direction(), Direction::Max);
        assert_eq!(
            limit.bounded_kind(),
            MeasureKind::Plain(Dimension::DIMENSIONLESS)
        );
        assert!(limit.parameters().is_empty());
        assert_eq!(limit.assumptions().len(), 1);
        let a = &limit.assumptions()[0];
        assert_eq!(a.id, AssumptionId::new("reference_area"));
        let ids: Vec<&str> = a.domain.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "unshrouded_rotor",
                "shrouded_total_capture_area",
                "shrouded_rotor_area_only"
            ]
        );
        assert!(a.domain.iter().all(|o| o.value.is_none()));
    }

    #[test]
    fn bound_is_sixteen_twenty_sevenths_for_the_two_valid_options() {
        for option in ["unshrouded_rotor", "shrouded_total_capture_area"] {
            let b = bound_of(option).expect("valid option has a bound");
            assert!(
                approx_eq(b.lo(), 16.0 / 27.0),
                "{option}: lo = {:?}",
                b.lo()
            );
            assert!(
                approx_eq(b.hi(), 16.0 / 27.0),
                "{option}: hi = {:?}",
                b.hi()
            );
        }
    }

    #[test]
    fn bound_is_exactly_the_f64_fraction_not_a_rounded_decimal() {
        let expected = 16.0_f64 / 27.0;
        let b = bound_of("unshrouded_rotor").expect("valid option has a bound");
        assert_eq!(b.lo(), expected);
        assert_eq!(b.hi(), expected);
        assert_ne!(b.hi(), 0.59);
    }

    #[test]
    fn rotor_area_only_is_not_applicable() {
        assert_eq!(
            bound_of("shrouded_rotor_area_only"),
            Err(EngineError::NotApplicable {
                limit_id: "betz".to_string(),
                assumption: "reference_area".to_string(),
                option: "shrouded_rotor_area_only".to_string(),
            })
        );
    }

    #[test]
    fn public_documentation_has_the_required_statements() {
        let doc: String = include_str!("betz.rs")
            .lines()
            .map(str::trim_start)
            .filter(|l| l.starts_with("///") || l.starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "Cp <= 16/27",
            "TODO(reference): to be supplied by the author",
            "reference_area",
            "unshrouded_rotor",
            "shrouded_total_capture_area",
            "shrouded_rotor_area_only",
            "NotApplicable",
            "INSUFFICIENT_DATA",
            "reference area not stated",
            "model bound",
            "inconsistent with limit",
        ] {
            assert!(doc.contains(needle), "documentation lacks: {needle}");
        }
    }

    #[test]
    fn unknown_option_is_a_typed_error() {
        assert!(matches!(
            bound_of("something_else"),
            Err(EngineError::ConstraintOutsideDomain { .. })
        ));
    }

    #[test]
    fn missing_assumption_is_a_typed_error() {
        let empty = Configuration::from_map(BTreeMap::new());
        assert_eq!(
            Betz::new().bound(&empty, &ParameterEnclosure::default()),
            Err(EngineError::UnknownAssumption("reference_area".to_string()))
        );
    }
}
