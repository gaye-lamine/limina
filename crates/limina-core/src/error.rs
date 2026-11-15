//! Typed errors of the engine.

use crate::dimension::Dimension;

/// Every error the engine can return. The engine never panics on bad input
/// and never silently converts between incompatible quantities.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EngineError {
    /// Two quantities with different dimensions were compared or combined.
    #[error("incompatible dimensions: expected {expected:?}, found {found:?}")]
    IncompatibleDimension {
        /// Dimension required by the operation.
        expected: Dimension,
        /// Dimension actually supplied.
        found: Dimension,
    },

    /// An absolute temperature was mixed with a temperature difference.
    #[error("absolute temperature and temperature difference cannot be mixed")]
    TemperatureKindMismatch,

    /// The unit `K` was used where the kind of temperature is unknown.
    #[error("unit K is ambiguous: use AbsoluteTemperature or TemperatureDifference")]
    TemperatureKindRequired,

    /// The unit symbol is not in the unit table.
    #[error("unknown unit: {0}")]
    UnknownUnit(String),

    /// An absolute temperature below 0 K was supplied.
    #[error("absolute temperature below 0 K: {0}")]
    NegativeAbsoluteTemperature(f64),

    /// A value is NaN or infinite.
    #[error("non-finite value for {field}")]
    NonFiniteValue {
        /// Name of the offending field.
        field: &'static str,
    },

    /// An interval was built with `lo > hi`.
    #[error("inverted interval")]
    InvertedInterval,

    /// An interval was divided by an interval that contains zero.
    #[error("division by an interval containing zero")]
    DivisionByIntervalContainingZero,
}

#[cfg(test)]
mod tests {
    use super::EngineError;
    use crate::dimension::Dimension;

    #[test]
    fn display_names_the_problem() {
        let err = EngineError::UnknownUnit("furlong".to_string());
        assert_eq!(err.to_string(), "unknown unit: furlong");

        let err = EngineError::NonFiniteValue { field: "value" };
        assert_eq!(err.to_string(), "non-finite value for value");
    }

    #[test]
    fn incompatible_dimension_carries_both_dimensions() {
        let expected = Dimension::new([2, 1, -3, 0, 0, 0, 0]);
        let found = Dimension::new([2, 1, -2, 0, 0, 0, 0]);
        let err = EngineError::IncompatibleDimension { expected, found };
        assert_eq!(err, EngineError::IncompatibleDimension { expected, found });
        assert!(err.to_string().starts_with("incompatible dimensions"));
    }
}
