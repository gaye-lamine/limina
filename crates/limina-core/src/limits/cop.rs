//! The Carnot bound on the coefficient of performance of a heat pump or
//! refrigerator.

use crate::dimension::Dimension;
use crate::error::EngineError;
use crate::interval::Interval;
use crate::limit::{
    Assumption, AssumptionId, AssumptionOption, Configuration, Direction, Limit, OptionId,
    ParameterEnclosure, ParameterId, ParameterSpec,
};
use crate::quantity::MeasureKind;

const LIMIT_ID: &str = "cop";
const MODE: &str = "mode";
const HEATING: &str = "heating";
const COOLING: &str = "cooling";
const TH: &str = "Th";
const TC: &str = "Tc";

fn parameter(params: &ParameterEnclosure, id: &str) -> Result<Interval, EngineError> {
    params
        .get(&ParameterId::new(id))
        .ok_or_else(|| EngineError::UnknownParameter(id.to_string()))
}

/// The Carnot bound on the coefficient of performance `COP` of a heat pump
/// or refrigerator working between two reservoirs.
///
/// Formulas, with `Th` and `Tc` the absolute temperatures in kelvin of the
/// hot and cold reservoirs:
/// - heating: `COP <= Th/(Th-Tc)`;
/// - cooling: `COP <= Tc/(Th-Tc)`.
///
/// `COP` is dimensionless and the limit is a Max bound.
///
/// Epistemic status: theorem (second law of thermodynamics).
///
/// Validity: `Th > Tc > 0`. If the parameter enclosures do not guarantee it,
/// `bound` returns `EngineError::InvalidTemperatureOrder` and computes no
/// bound. That error is never a verdict.
///
/// Assumption `mode`, with two options:
/// - `heating`: the useful output is the heat delivered to the hot reservoir;
///   the bound is `Th/(Th-Tc)`;
/// - `cooling`: the useful output is the heat removed from the cold
///   reservoir; the bound is `Tc/(Th-Tc)`.
///
/// When `mode` is not stated, both configurations stay in the set, so a
/// claim between the two bounds is CONDITIONAL on `mode = heating`.
///
/// Parameters `Th` and `Tc` are typed `AbsoluteTemperature`, required, and
/// have no declared plausible domain. A missing temperature therefore gives
/// INSUFFICIENT_DATA with the missing parameters listed, produced by the
/// engine and not by this limit.
///
/// A claim above the bound is reported as "inconsistent with limit cop under
/// assumptions mode = heating" (or `cooling`).
///
/// Reference: // TODO(reference): to be supplied by the author
// TODO(reference): to be supplied by the author
#[derive(Debug, Clone)]
pub struct Cop {
    assumptions: Vec<Assumption>,
    parameters: Vec<ParameterSpec>,
}

impl Cop {
    /// Builds the limit.
    pub fn new() -> Cop {
        let spec = |id: &str| ParameterSpec {
            id: ParameterId::new(id),
            kind: MeasureKind::AbsoluteTemperature,
            required: true,
            plausible: None,
        };
        let option = |id: &str| AssumptionOption {
            id: OptionId::new(id),
            value: None,
        };
        Cop {
            assumptions: vec![Assumption {
                id: AssumptionId::new(MODE),
                domain: vec![option(HEATING), option(COOLING)],
            }],
            parameters: vec![spec(TH), spec(TC)],
        }
    }
}

impl Default for Cop {
    fn default() -> Cop {
        Cop::new()
    }
}

/// Which of the two bounds a configuration asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Heating,
    Cooling,
}

fn mode_of(config: &Configuration) -> Result<Mode, EngineError> {
    let option = config
        .option(&AssumptionId::new(MODE))
        .ok_or_else(|| EngineError::UnknownAssumption(MODE.to_string()))?;
    match option.as_str() {
        HEATING => Ok(Mode::Heating),
        COOLING => Ok(Mode::Cooling),
        other => Err(EngineError::ConstraintOutsideDomain {
            assumption: MODE.to_string(),
            option: other.to_string(),
        }),
    }
}

impl Limit for Cop {
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
        let mode = mode_of(config)?;
        let th = parameter(params, TH)?;
        let tc = parameter(params, TC)?;
        if !(tc.lo() > 0.0 && th.lo() > tc.hi()) {
            return Err(EngineError::InvalidTemperatureOrder {
                limit_id: LIMIT_ID.to_string(),
            });
        }
        // Both bounds decrease in Th and increase in Tc, so the exact range
        // over the enclosures is taken at the corners. Point enclosures give
        // the f64 value of the formula.
        let (num_lo, num_hi) = match mode {
            Mode::Heating => (th.hi(), th.lo()),
            Mode::Cooling => (tc.lo(), tc.hi()),
        };
        Interval::new(num_lo / (th.hi() - tc.lo()), num_hi / (th.lo() - tc.hi()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn config(option: &str) -> Configuration {
        let mut map = BTreeMap::new();
        map.insert(AssumptionId::new(MODE), OptionId::new(option));
        Configuration::from_map(map)
    }

    fn enclosure(th: (f64, f64), tc: (f64, f64)) -> ParameterEnclosure {
        let iv = |(lo, hi): (f64, f64)| Interval::new(lo, hi).expect("valid interval");
        let mut map = BTreeMap::new();
        map.insert(ParameterId::new(TH), iv(th));
        map.insert(ParameterId::new(TC), iv(tc));
        ParameterEnclosure::new(map)
    }

    fn bound_at(option: &str, th: f64, tc: f64) -> Result<Interval, EngineError> {
        Cop::new().bound(&config(option), &enclosure((th, th), (tc, tc)))
    }

    fn assert_point(option: &str, th: f64, tc: f64, expected: f64) {
        let b = bound_at(option, th, tc).expect("valid temperatures");
        assert_eq!(b.lo(), expected, "{option} Th={th} Tc={tc}");
        assert_eq!(b.hi(), expected, "{option} Th={th} Tc={tc}");
    }

    fn order_error() -> EngineError {
        EngineError::InvalidTemperatureOrder {
            limit_id: "cop".to_string(),
        }
    }

    #[test]
    fn declarations_match_the_spec() {
        let limit = Cop::new();
        assert_eq!(limit.id(), "cop");
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
        assert_eq!(a.id, AssumptionId::new("mode"));
        let options: Vec<&str> = a.domain.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(options, ["heating", "cooling"]);
        assert!(a.domain.iter().all(|o| o.value.is_none()));
    }

    #[test]
    fn golden_300_280() {
        assert_eq!(300.0_f64 / (300.0 - 280.0), 15.0);
        assert_eq!(280.0_f64 / (300.0 - 280.0), 14.0);
        assert_point("heating", 300.0, 280.0, 15.0);
        assert_point("cooling", 300.0, 280.0, 14.0);
    }

    #[test]
    fn golden_320_280() {
        assert_eq!(320.0_f64 / (320.0 - 280.0), 8.0);
        assert_eq!(280.0_f64 / (320.0 - 280.0), 7.0);
        assert_point("heating", 320.0, 280.0, 8.0);
        assert_point("cooling", 320.0, 280.0, 7.0);
    }

    #[test]
    fn golden_300_250() {
        assert_eq!(300.0_f64 / (300.0 - 250.0), 6.0);
        assert_eq!(250.0_f64 / (300.0 - 250.0), 5.0);
        assert_point("heating", 300.0, 250.0, 6.0);
        assert_point("cooling", 300.0, 250.0, 5.0);
    }

    #[test]
    fn th_not_above_tc_is_a_typed_error() {
        for option in ["heating", "cooling"] {
            assert_eq!(bound_at(option, 300.0, 300.0), Err(order_error()));
            assert_eq!(bound_at(option, 280.0, 300.0), Err(order_error()));
        }
    }

    #[test]
    fn non_positive_tc_is_a_typed_error() {
        assert_eq!(bound_at("heating", 300.0, 0.0), Err(order_error()));
    }

    #[test]
    fn overlapping_enclosures_are_a_typed_error() {
        let params = enclosure((290.0, 320.0), (280.0, 295.0));
        assert_eq!(
            Cop::new().bound(&config("heating"), &params),
            Err(order_error())
        );
    }

    #[test]
    fn enclosure_bound_uses_the_corners() {
        let params = enclosure((300.0, 320.0), (250.0, 280.0));
        let heating = Cop::new()
            .bound(&config("heating"), &params)
            .expect("valid enclosure");
        assert_eq!(heating.lo(), 320.0 / (320.0 - 250.0));
        assert_eq!(heating.hi(), 300.0 / (300.0 - 280.0));
        let cooling = Cop::new()
            .bound(&config("cooling"), &params)
            .expect("valid enclosure");
        assert_eq!(cooling.lo(), 250.0 / (320.0 - 250.0));
        assert_eq!(cooling.hi(), 280.0 / (300.0 - 280.0));
    }

    #[test]
    fn missing_parameter_is_a_typed_error() {
        assert_eq!(
            Cop::new().bound(&config("heating"), &ParameterEnclosure::default()),
            Err(EngineError::UnknownParameter("Th".to_string()))
        );
    }

    #[test]
    fn unknown_option_is_a_typed_error() {
        let params = enclosure((300.0, 300.0), (280.0, 280.0));
        assert!(matches!(
            Cop::new().bound(&config("something_else"), &params),
            Err(EngineError::ConstraintOutsideDomain { .. })
        ));
    }

    #[test]
    fn missing_assumption_is_a_typed_error() {
        let params = enclosure((300.0, 300.0), (280.0, 280.0));
        let empty = Configuration::from_map(BTreeMap::new());
        assert_eq!(
            Cop::new().bound(&empty, &params),
            Err(EngineError::UnknownAssumption("mode".to_string()))
        );
    }

    #[test]
    fn public_documentation_has_the_required_statements() {
        let doc: String = include_str!("cop.rs")
            .lines()
            .map(str::trim_start)
            .filter(|l| l.starts_with("///") || l.starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "COP <= Th/(Th-Tc)",
            "COP <= Tc/(Th-Tc)",
            "Th > Tc > 0",
            "mode",
            "heating",
            "cooling",
            "theorem",
            "second law of thermodynamics",
            "InvalidTemperatureOrder",
            "INSUFFICIENT_DATA",
            "TODO(reference): to be supplied by the author",
            "inconsistent with limit",
        ] {
            assert!(doc.contains(needle), "documentation lacks: {needle}");
        }
    }
}
