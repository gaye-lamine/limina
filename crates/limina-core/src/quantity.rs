//! Typed physical quantities with a runtime dimension.
//!
//! Values are stored in SI base units. Temperatures in kelvin are not plain
//! quantities: the unit `K` is rejected here and must go through the
//! temperature types, so an absolute temperature and a temperature difference
//! are never confused.

use crate::dimension::Dimension;
use crate::error::EngineError;
use crate::units::lookup_unit;

/// Dimension Θ¹ (thermodynamic temperature). Never the dimension of a `Quantity`.
const TEMPERATURE: Dimension = Dimension::new([0, 0, 0, 0, 1, 0, 0]);

/// A finite value in SI base units with a runtime dimension.
///
/// Invariants: the value is finite, and the dimension is never exactly
/// temperature (Θ¹).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    si_value: f64,
    dimension: Dimension,
}

impl Quantity {
    /// Builds a quantity from a value in `unit`.
    ///
    /// Errors: `UnknownUnit` if `unit` is not in the unit table,
    /// `TemperatureKindRequired` for `K`, `NonFiniteValue` if `value` or its
    /// SI conversion is not finite.
    pub fn from_unit(value: f64, unit: &str) -> Result<Quantity, EngineError> {
        let entry = lookup_unit(unit)?;
        if entry.dimension == TEMPERATURE {
            return Err(EngineError::TemperatureKindRequired);
        }
        let value = finite(value)?;
        let si_value = finite(value * entry.factor_to_si)?;
        Ok(Quantity {
            si_value,
            dimension: entry.dimension,
        })
    }

    /// Builds a quantity from a value already in SI base units.
    ///
    /// Errors: `NonFiniteValue` if `value_si` is not finite,
    /// `TemperatureKindRequired` if `dimension` is temperature (Θ¹).
    pub fn from_si(value_si: f64, dimension: Dimension) -> Result<Quantity, EngineError> {
        if dimension == TEMPERATURE {
            return Err(EngineError::TemperatureKindRequired);
        }
        Ok(Quantity {
            si_value: finite(value_si)?,
            dimension,
        })
    }

    /// Returns the value expressed in `unit`.
    ///
    /// Errors: `UnknownUnit` if `unit` is not in the unit table,
    /// `IncompatibleDimension` if `unit` has another dimension. No conversion
    /// between dimensions is ever made.
    pub fn to_unit(&self, unit: &str) -> Result<f64, EngineError> {
        let entry = lookup_unit(unit)?;
        if entry.dimension != self.dimension {
            return Err(EngineError::IncompatibleDimension {
                expected: self.dimension,
                found: entry.dimension,
            });
        }
        Ok(self.si_value / entry.factor_to_si)
    }

    /// Value in SI base units.
    pub fn si_value(&self) -> f64 {
        self.si_value
    }

    /// Dimension of the quantity.
    pub fn dimension(&self) -> Dimension {
        self.dimension
    }
}

/// Absolute (thermodynamic) temperature in kelvin.
///
/// Invariant: finite and `>= 0`. There is no Celsius or Fahrenheit
/// constructor, so no silent offset conversion exists.
///
/// Only these operations exist:
/// `AbsoluteTemperature - AbsoluteTemperature = TemperatureDifference`, and
/// `AbsoluteTemperature::checked_add(TemperatureDifference)`.
/// Adding two absolute temperatures does not compile:
///
/// ```compile_fail
/// use limina_core::AbsoluteTemperature;
/// let a = AbsoluteTemperature::from_kelvin(300.0).unwrap();
/// let _ = a + a;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AbsoluteTemperature(f64);

/// Difference of two temperatures, in kelvin. Invariant: finite. May be negative.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TemperatureDifference(f64);

impl AbsoluteTemperature {
    /// Errors: `NonFiniteValue` if `k` is not finite,
    /// `NegativeAbsoluteTemperature` if `k < 0`.
    pub fn from_kelvin(k: f64) -> Result<AbsoluteTemperature, EngineError> {
        let k = finite_field(k, "kelvin")?;
        if k < 0.0 {
            return Err(EngineError::NegativeAbsoluteTemperature(k));
        }
        Ok(AbsoluteTemperature(k))
    }

    /// Value in kelvin.
    pub fn kelvin(&self) -> f64 {
        self.0
    }

    /// `self + delta`. Errors: `NegativeAbsoluteTemperature` if the result is
    /// below 0 K, `NonFiniteValue` if it overflows.
    pub fn checked_add(
        self,
        delta: TemperatureDifference,
    ) -> Result<AbsoluteTemperature, EngineError> {
        AbsoluteTemperature::from_kelvin(self.0 + delta.0)
    }
}

impl std::ops::Sub for AbsoluteTemperature {
    type Output = TemperatureDifference;

    /// Both operands are finite and `>= 0`, so the difference is finite.
    fn sub(self, rhs: AbsoluteTemperature) -> TemperatureDifference {
        TemperatureDifference(self.0 - rhs.0)
    }
}

impl TemperatureDifference {
    /// Errors: `NonFiniteValue` if `k` is not finite.
    pub fn from_kelvin(k: f64) -> Result<TemperatureDifference, EngineError> {
        Ok(TemperatureDifference(finite_field(k, "kelvin")?))
    }

    /// Value in kelvin.
    pub fn kelvin(&self) -> f64 {
        self.0
    }
}

/// A value the engine receives and compares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Measure {
    /// Any quantity other than a temperature.
    Plain(Quantity),
    /// An absolute temperature.
    AbsoluteTemperature(AbsoluteTemperature),
    /// A temperature difference.
    TemperatureDifference(TemperatureDifference),
}

/// The kind of a `Measure`. Two measures are comparable only if their kinds
/// are equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasureKind {
    /// A plain quantity of the given dimension.
    Plain(Dimension),
    /// An absolute temperature.
    AbsoluteTemperature,
    /// A temperature difference.
    TemperatureDifference,
}

impl Measure {
    /// Kind of this measure.
    pub fn kind(&self) -> MeasureKind {
        match self {
            Measure::Plain(q) => MeasureKind::Plain(q.dimension()),
            Measure::AbsoluteTemperature(_) => MeasureKind::AbsoluteTemperature,
            Measure::TemperatureDifference(_) => MeasureKind::TemperatureDifference,
        }
    }

    /// Value in SI base units (kelvin for temperatures).
    pub fn si_value(&self) -> f64 {
        match self {
            Measure::Plain(q) => q.si_value(),
            Measure::AbsoluteTemperature(t) => t.kelvin(),
            Measure::TemperatureDifference(d) => d.kelvin(),
        }
    }
}

impl MeasureKind {
    /// Kind of the uncertainty of a value of this kind.
    /// `AbsoluteTemperature` gives `TemperatureDifference`; every other kind
    /// gives itself.
    pub fn uncertainty_kind(&self) -> MeasureKind {
        match self {
            MeasureKind::AbsoluteTemperature => MeasureKind::TemperatureDifference,
            other => *other,
        }
    }
}

/// Checks that `found` has the kind `expected`, without any conversion.
///
/// Errors: `TemperatureKindMismatch` for an absolute temperature against a
/// temperature difference, `IncompatibleDimension` otherwise.
// Used by the engine input validation (T15); until then only by tests.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn check_same_kind(
    expected: MeasureKind,
    found: MeasureKind,
) -> Result<(), EngineError> {
    if expected == found {
        return Ok(());
    }
    match (expected, found) {
        (MeasureKind::AbsoluteTemperature, MeasureKind::TemperatureDifference)
        | (MeasureKind::TemperatureDifference, MeasureKind::AbsoluteTemperature) => {
            Err(EngineError::TemperatureKindMismatch)
        }
        _ => Err(EngineError::IncompatibleDimension {
            expected: dimension_of(expected),
            found: dimension_of(found),
        }),
    }
}

fn dimension_of(kind: MeasureKind) -> Dimension {
    match kind {
        MeasureKind::Plain(d) => d,
        MeasureKind::AbsoluteTemperature | MeasureKind::TemperatureDifference => TEMPERATURE,
    }
}

fn finite(x: f64) -> Result<f64, EngineError> {
    finite_field(x, "value")
}

fn finite_field(x: f64, field: &'static str) -> Result<f64, EngineError> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(EngineError::NonFiniteValue { field })
    }
}

#[cfg(test)]
mod tests {
    use super::{Quantity, TEMPERATURE};
    use crate::dimension::Dimension;
    use crate::error::EngineError;

    const POWER: Dimension = Dimension::new([2, 1, -3, 0, 0, 0, 0]);
    const ENERGY: Dimension = Dimension::new([2, 1, -2, 0, 0, 0, 0]);

    fn q(value: f64, unit: &str) -> Quantity {
        Quantity::from_unit(value, unit).expect("valid quantity")
    }

    #[test]
    fn kilowatt_converts_to_watt() {
        let p = q(2.5, "kW");
        assert_eq!(p.si_value(), 2500.0);
        assert_eq!(p.dimension(), POWER);
        assert_eq!(p.to_unit("W"), Ok(2500.0));
        assert_eq!(q(3.0, "MW").to_unit("kW"), Ok(3000.0));
    }

    #[test]
    fn kilowatt_hour_converts_to_joule() {
        let e = q(1.0, "kWh");
        assert_eq!(e.si_value(), 3.6e6);
        assert_eq!(e.dimension(), ENERGY);
        assert_eq!(q(7.2e6, "J").to_unit("kWh"), Ok(2.0));
    }

    #[test]
    fn unknown_unit_is_rejected() {
        assert_eq!(
            Quantity::from_unit(1.0, "hp"),
            Err(EngineError::UnknownUnit("hp".to_string()))
        );
        assert_eq!(
            q(1.0, "W").to_unit("hp"),
            Err(EngineError::UnknownUnit("hp".to_string()))
        );
    }

    #[test]
    fn kelvin_requires_a_temperature_type() {
        assert_eq!(
            Quantity::from_unit(300.0, "K"),
            Err(EngineError::TemperatureKindRequired)
        );
        assert_eq!(
            Quantity::from_si(300.0, TEMPERATURE),
            Err(EngineError::TemperatureKindRequired)
        );
    }

    #[test]
    fn non_finite_values_are_rejected() {
        let err = Err(EngineError::NonFiniteValue { field: "value" });
        assert_eq!(Quantity::from_unit(f64::NAN, "W"), err);
        assert_eq!(Quantity::from_unit(f64::INFINITY, "J"), err);
        assert_eq!(Quantity::from_si(f64::NEG_INFINITY, POWER), err);
        // Finite input whose SI conversion overflows.
        assert_eq!(Quantity::from_unit(f64::MAX, "MW"), err);
    }

    #[test]
    fn conversion_to_another_dimension_is_rejected() {
        assert_eq!(
            q(1.0, "kW").to_unit("kWh"),
            Err(EngineError::IncompatibleDimension {
                expected: POWER,
                found: ENERGY,
            })
        );
        assert!(matches!(
            q(1.0, "m").to_unit("K"),
            Err(EngineError::IncompatibleDimension { .. })
        ));
    }

    #[test]
    fn from_si_keeps_value_and_dimension() {
        let x = Quantity::from_si(-4.0, Dimension::DIMENSIONLESS).expect("valid");
        assert_eq!(x.si_value(), -4.0);
        assert_eq!(x.to_unit("dimensionless"), Ok(-4.0));
    }
}

#[cfg(test)]
mod temperature_tests {
    use super::{
        check_same_kind, AbsoluteTemperature, Measure, MeasureKind, Quantity,
        TemperatureDifference, TEMPERATURE,
    };
    use crate::dimension::Dimension;
    use crate::error::EngineError;

    const POWER: Dimension = Dimension::new([2, 1, -3, 0, 0, 0, 0]);
    const ENERGY: Dimension = Dimension::new([2, 1, -2, 0, 0, 0, 0]);

    fn abs(k: f64) -> AbsoluteTemperature {
        AbsoluteTemperature::from_kelvin(k).expect("valid absolute temperature")
    }

    fn diff(k: f64) -> TemperatureDifference {
        TemperatureDifference::from_kelvin(k).expect("valid temperature difference")
    }

    #[test]
    fn negative_absolute_temperature_is_rejected() {
        assert_eq!(
            AbsoluteTemperature::from_kelvin(-1.0),
            Err(EngineError::NegativeAbsoluteTemperature(-1.0))
        );
        assert_eq!(abs(0.0).kelvin(), 0.0);
    }

    #[test]
    fn non_finite_temperatures_are_rejected() {
        let err = Err(EngineError::NonFiniteValue { field: "kelvin" });
        assert_eq!(AbsoluteTemperature::from_kelvin(f64::NAN), err);
        assert_eq!(AbsoluteTemperature::from_kelvin(f64::INFINITY), err);
        assert_eq!(
            TemperatureDifference::from_kelvin(f64::NEG_INFINITY),
            Err(EngineError::NonFiniteValue { field: "kelvin" })
        );
    }

    #[test]
    fn difference_may_be_negative() {
        assert_eq!(diff(-20.0).kelvin(), -20.0);
    }

    #[test]
    fn absolute_minus_absolute_is_a_difference() {
        assert_eq!(abs(300.0) - abs(280.0), diff(20.0));
        assert_eq!(abs(280.0) - abs(300.0), diff(-20.0));
    }

    #[test]
    fn absolute_plus_difference_is_checked() {
        assert_eq!(abs(300.0).checked_add(diff(-20.0)), Ok(abs(280.0)));
        assert_eq!(
            abs(10.0).checked_add(diff(-20.0)),
            Err(EngineError::NegativeAbsoluteTemperature(-10.0))
        );
        assert_eq!(
            abs(f64::MAX).checked_add(diff(f64::MAX)),
            Err(EngineError::NonFiniteValue { field: "kelvin" })
        );
    }

    #[test]
    fn measure_reports_kind_and_si_value() {
        let p = Measure::Plain(Quantity::from_unit(2.0, "kW").expect("valid"));
        assert_eq!(p.kind(), MeasureKind::Plain(POWER));
        assert_eq!(p.si_value(), 2000.0);

        let t = Measure::AbsoluteTemperature(abs(300.0));
        assert_eq!(t.kind(), MeasureKind::AbsoluteTemperature);
        assert_eq!(t.si_value(), 300.0);

        let d = Measure::TemperatureDifference(diff(-5.0));
        assert_eq!(d.kind(), MeasureKind::TemperatureDifference);
        assert_eq!(d.si_value(), -5.0);
    }

    #[test]
    fn uncertainty_of_an_absolute_temperature_is_a_difference() {
        assert_eq!(
            MeasureKind::AbsoluteTemperature.uncertainty_kind(),
            MeasureKind::TemperatureDifference
        );
        assert_eq!(
            MeasureKind::TemperatureDifference.uncertainty_kind(),
            MeasureKind::TemperatureDifference
        );
        assert_eq!(
            MeasureKind::Plain(POWER).uncertainty_kind(),
            MeasureKind::Plain(POWER)
        );
    }

    #[test]
    fn absolute_against_difference_is_a_kind_mismatch() {
        assert_eq!(
            check_same_kind(
                MeasureKind::AbsoluteTemperature,
                MeasureKind::TemperatureDifference
            ),
            Err(EngineError::TemperatureKindMismatch)
        );
        assert_eq!(
            check_same_kind(
                MeasureKind::TemperatureDifference,
                MeasureKind::AbsoluteTemperature
            ),
            Err(EngineError::TemperatureKindMismatch)
        );
    }

    #[test]
    fn different_dimensions_are_incompatible_and_never_converted() {
        assert_eq!(
            check_same_kind(MeasureKind::Plain(POWER), MeasureKind::Plain(ENERGY)),
            Err(EngineError::IncompatibleDimension {
                expected: POWER,
                found: ENERGY,
            })
        );
        assert_eq!(
            check_same_kind(MeasureKind::Plain(POWER), MeasureKind::AbsoluteTemperature),
            Err(EngineError::IncompatibleDimension {
                expected: POWER,
                found: TEMPERATURE,
            })
        );
    }

    #[test]
    fn same_kind_is_accepted() {
        for kind in [
            MeasureKind::Plain(POWER),
            MeasureKind::AbsoluteTemperature,
            MeasureKind::TemperatureDifference,
        ] {
            assert_eq!(check_same_kind(kind, kind), Ok(()));
        }
    }
}
