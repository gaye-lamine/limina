//! The Carnot limit on the thermal efficiency of a heat engine.

use crate::dimension::Dimension;
use crate::error::EngineError;
use crate::interval::Interval;
use crate::limit::{
    Assumption, AssumptionId, AssumptionOption, Configuration, Direction, Limit, OptionId,
    ParameterEnclosure, ParameterId, ParameterSpec,
};
use crate::quantity::MeasureKind;

const LIMIT_ID: &str = "carnot";
const CYCLE_MODEL: &str = "cycle_model";
const IDEAL_REVERSIBLE: &str = "ideal_reversible_two_reservoirs";
const TH: &str = "Th";
const TC: &str = "Tc";

fn parameter(params: &ParameterEnclosure, id: &str) -> Result<Interval, EngineError> {
    params
        .get(&ParameterId::new(id))
        .ok_or_else(|| EngineError::UnknownParameter(id.to_string()))
}

/// The Carnot limit on the thermal efficiency `eta` of a heat engine.
///
/// Formula: `eta <= 1 - Tc/Th`, with `Th` and `Tc` the absolute temperatures
/// in kelvin of the hot and cold reservoirs. `eta` is dimensionless and the
/// limit is a Max bound.
///
/// Epistemic status: theorem (second law of thermodynamics).
///
/// Validity: `Th > Tc > 0`. If the parameter enclosures do not guarantee it,
/// `bound` returns `EngineError::InvalidTemperatureOrder` and computes no
/// bound. That error is never a verdict.
///
/// Assumption `cycle_model`, with one option:
/// - `ideal_reversible_two_reservoirs`: an ideal reversible cycle between two
///   reservoirs. The bound is the Carnot bound.
///
/// Parameters `Th` and `Tc` are typed `AbsoluteTemperature`, required, and
/// have no declared plausible domain. A missing temperature therefore gives
/// INSUFFICIENT_DATA with the missing parameters listed, produced by the
/// engine and not by this limit.
///
/// Celsius is never converted silently: temperatures must arrive as
/// `AbsoluteTemperature`, and any conversion is the caller's explicit job.
///
/// A claim above the bound is reported as "inconsistent with limit carnot
/// under assumptions cycle_model = ideal_reversible_two_reservoirs".
///
/// Reference: // TODO(reference): to be supplied by the author
// TODO(reference): to be supplied by the author
#[derive(Debug, Clone)]
pub struct Carnot {
    assumptions: Vec<Assumption>,
    parameters: Vec<ParameterSpec>,
}

impl Carnot {
    /// Builds the limit.
    pub fn new() -> Carnot {
        let spec = |id: &str| ParameterSpec {
            id: ParameterId::new(id),
            kind: MeasureKind::AbsoluteTemperature,
            required: true,
            plausible: None,
        };
        Carnot {
            assumptions: vec![Assumption {
                id: AssumptionId::new(CYCLE_MODEL),
                domain: vec![AssumptionOption {
                    id: OptionId::new(IDEAL_REVERSIBLE),
                    value: None,
                }],
            }],
            parameters: vec![spec(TH), spec(TC)],
        }
    }
}

impl Default for Carnot {
    fn default() -> Carnot {
        Carnot::new()
    }
}

impl Limit for Carnot {
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
        params: &ParameterEnclosure,
    ) -> Result<Interval, EngineError> {
        let assumption = AssumptionId::new(CYCLE_MODEL);
        let option = config
            .option(&assumption)
            .ok_or_else(|| EngineError::UnknownAssumption(CYCLE_MODEL.to_string()))?;
        if option.as_str() != IDEAL_REVERSIBLE {
            return Err(EngineError::ConstraintOutsideDomain {
                assumption: CYCLE_MODEL.to_string(),
                option: option.as_str().to_string(),
            });
        }
        let th = parameter(params, TH)?;
        let tc = parameter(params, TC)?;
        if !(tc.lo() > 0.0 && th.lo() > tc.hi()) {
            return Err(EngineError::InvalidTemperatureOrder {
                limit_id: LIMIT_ID.to_string(),
            });
        }
        // 1 - Tc/Th is increasing in Th and decreasing in Tc, so the interval
        // quotient already takes its extremes at the enclosure endpoints.
        Interval::point(1.0)?.sub(tc.div(th)?)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn config(option: &str) -> Configuration {
        let mut map = BTreeMap::new();
        map.insert(AssumptionId::new(CYCLE_MODEL), OptionId::new(option));
        Configuration::from_map(map)
    }

    fn enclosure(th: (f64, f64), tc: (f64, f64)) -> ParameterEnclosure {
        let iv = |(lo, hi): (f64, f64)| Interval::new(lo, hi).expect("valid interval");
        let mut map = BTreeMap::new();
        map.insert(ParameterId::new(TH), iv(th));
        map.insert(ParameterId::new(TC), iv(tc));
        ParameterEnclosure::new(map)
    }

    fn bound_at(th: f64, tc: f64) -> Result<Interval, EngineError> {
        Carnot::new().bound(&config(IDEAL_REVERSIBLE), &enclosure((th, th), (tc, tc)))
    }

    fn order_error() -> EngineError {
        EngineError::InvalidTemperatureOrder {
            limit_id: "carnot".to_string(),
        }
    }

    #[test]
    fn declarations_match_the_spec() {
        let limit = Carnot::new();
        assert_eq!(limit.id(), "carnot");
        assert_eq!(limit.direction(), Direction::Max);
        assert_eq!(
            limit.bounded_kind(),
            MeasureKind::Plain(Dimension::DIMENSIONLESS)
        );
        let ids: Vec<&str> = limit.parameters().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["Th", "Tc"]);
        for p in limit.parameters() {
            assert_eq!(p.kind, MeasureKind::AbsoluteTemperature);
            assert!(p.required);
            assert!(p.plausible.is_none(), "{} declares a domain", p.id.as_str());
        }
        assert_eq!(limit.assumptions().len(), 1);
        let a = &limit.assumptions()[0];
        assert_eq!(a.id, AssumptionId::new("cycle_model"));
        assert_eq!(a.domain.len(), 1);
        assert_eq!(a.domain[0].id.as_str(), "ideal_reversible_two_reservoirs");
        assert!(a.domain[0].value.is_none());
    }

    #[test]
    fn golden_600_300_is_exactly_one_half() {
        let expected = 1.0_f64 - 300.0_f64 / 600.0_f64;
        let b = bound_at(600.0, 300.0).expect("valid temperatures");
        assert_eq!(expected, 0.5);
        assert_eq!(b.lo(), expected);
        assert_eq!(b.hi(), expected);
    }

    #[test]
    fn golden_500_250_is_exactly_one_half() {
        let expected = 1.0_f64 - 250.0_f64 / 500.0_f64;
        let b = bound_at(500.0, 250.0).expect("valid temperatures");
        assert_eq!(expected, 0.5);
        assert_eq!(b.lo(), expected);
        assert_eq!(b.hi(), expected);
    }

    #[test]
    fn th_not_above_tc_is_a_typed_error() {
        assert_eq!(bound_at(300.0, 300.0), Err(order_error()));
        assert_eq!(bound_at(300.0, 600.0), Err(order_error()));
    }

    #[test]
    fn non_positive_tc_is_a_typed_error() {
        assert_eq!(bound_at(300.0, 0.0), Err(order_error()));
    }

    #[test]
    fn overlapping_enclosures_are_a_typed_error() {
        let params = enclosure((250.0, 400.0), (300.0, 350.0));
        assert_eq!(
            Carnot::new().bound(&config(IDEAL_REVERSIBLE), &params),
            Err(order_error())
        );
    }

    #[test]
    fn enclosure_bound_uses_the_endpoints() {
        let params = enclosure((500.0, 600.0), (200.0, 300.0));
        let b = Carnot::new()
            .bound(&config(IDEAL_REVERSIBLE), &params)
            .expect("valid enclosure");
        assert_eq!(b.lo(), 1.0 - 300.0 / 500.0);
        assert_eq!(b.hi(), 1.0 - 200.0 / 600.0);
    }

    #[test]
    fn missing_parameter_is_a_typed_error() {
        assert_eq!(
            Carnot::new().bound(&config(IDEAL_REVERSIBLE), &ParameterEnclosure::default()),
            Err(EngineError::UnknownParameter("Th".to_string()))
        );
    }

    #[test]
    fn unknown_option_is_a_typed_error() {
        let params = enclosure((600.0, 600.0), (300.0, 300.0));
        assert!(matches!(
            Carnot::new().bound(&config("something_else"), &params),
            Err(EngineError::ConstraintOutsideDomain { .. })
        ));
    }

    #[test]
    fn missing_assumption_is_a_typed_error() {
        let params = enclosure((600.0, 600.0), (300.0, 300.0));
        let empty = Configuration::from_map(BTreeMap::new());
        assert_eq!(
            Carnot::new().bound(&empty, &params),
            Err(EngineError::UnknownAssumption("cycle_model".to_string()))
        );
    }

    #[test]
    fn public_documentation_has_the_required_statements() {
        let doc: String = include_str!("carnot.rs")
            .lines()
            .map(str::trim_start)
            .filter(|l| l.starts_with("///") || l.starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "eta <= 1 - Tc/Th",
            "Th > Tc > 0",
            "cycle_model",
            "ideal_reversible_two_reservoirs",
            "theorem",
            "second law of thermodynamics",
            "Celsius",
            "never converted",
            "InvalidTemperatureOrder",
            "INSUFFICIENT_DATA",
            "TODO(reference): to be supplied by the author",
            "inconsistent with limit",
        ] {
            assert!(doc.contains(needle), "documentation lacks: {needle}");
        }
    }
}
