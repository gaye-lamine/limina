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

    /// The coverage factor `k` is not finite or not `> 0`.
    #[error("invalid coverage factor (must be finite and > 0)")]
    InvalidCoverageFactor,

    /// `guard_rel` is outside `0 <= guard_rel < 1`, or `guard_abs` is negative
    /// or not finite (D-18, D-21).
    #[error(
        "invalid guard (guard_rel must be finite and 0 <= guard_rel < 1; guard_abs must be finite and >= 0)"
    )]
    InvalidGuard,

    /// `max_configurations` is 0 (D-21).
    #[error("invalid max_configurations (must be >= 1)")]
    InvalidMaxConfigurations,

    /// The uncertainty `u` is negative.
    #[error("negative uncertainty")]
    NegativeUncertainty,

    /// A limit returned a negative or non-finite bound (R-02.5).
    #[error("invalid bound for limit {limit_id}: negative or non-finite")]
    InvalidBound {
        /// Id of the limit that returned the bound.
        limit_id: String,
    },

    /// The constraints map names an assumption the limit does not declare.
    #[error("unknown assumption: {0}")]
    UnknownAssumption(String),

    /// A constraint names an option outside the declared domain (R-08.4).
    #[error("constraint on {assumption} names option {option} outside its declared domain")]
    ConstraintOutsideDomain {
        /// Id of the constrained assumption.
        assumption: String,
        /// Id of the option outside the domain.
        option: String,
    },

    /// A supplied parameter is not declared by the limit.
    #[error("unknown parameter: {0}")]
    UnknownParameter(String),

    /// The configuration set `S` is empty (D-07). Never a verdict.
    #[error("empty configuration set")]
    EmptyConfigurationSet,

    /// `|S|` exceeds `max_configurations` (D-17). Never a verdict.
    #[error("configuration set larger than the cap of {max}")]
    ConfigurationSetTooLarge {
        /// The cap that was exceeded.
        max: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::EngineError;
    use crate::dimension::Dimension;

    /// One value of every variant. `variant_index` has no wildcard arm, so
    /// adding a variant without updating this list does not compile.
    fn all_variants() -> Vec<EngineError> {
        let d = Dimension::DIMENSIONLESS;
        vec![
            EngineError::IncompatibleDimension {
                expected: d,
                found: d,
            },
            EngineError::TemperatureKindMismatch,
            EngineError::TemperatureKindRequired,
            EngineError::UnknownUnit("x".to_string()),
            EngineError::NegativeAbsoluteTemperature(-1.0),
            EngineError::NonFiniteValue { field: "value" },
            EngineError::InvertedInterval,
            EngineError::DivisionByIntervalContainingZero,
            EngineError::InvalidCoverageFactor,
            EngineError::InvalidGuard,
            EngineError::InvalidMaxConfigurations,
            EngineError::NegativeUncertainty,
            EngineError::InvalidBound {
                limit_id: "test-limit".to_string(),
            },
            EngineError::UnknownAssumption("a".to_string()),
            EngineError::ConstraintOutsideDomain {
                assumption: "a".to_string(),
                option: "o".to_string(),
            },
            EngineError::UnknownParameter("p".to_string()),
            EngineError::EmptyConfigurationSet,
            EngineError::ConfigurationSetTooLarge { max: 100_000 },
        ]
    }

    fn variant_index(e: &EngineError) -> usize {
        match e {
            EngineError::IncompatibleDimension { .. } => 0,
            EngineError::TemperatureKindMismatch => 1,
            EngineError::TemperatureKindRequired => 2,
            EngineError::UnknownUnit(_) => 3,
            EngineError::NegativeAbsoluteTemperature(_) => 4,
            EngineError::NonFiniteValue { .. } => 5,
            EngineError::InvertedInterval => 6,
            EngineError::DivisionByIntervalContainingZero => 7,
            EngineError::InvalidCoverageFactor => 8,
            EngineError::InvalidGuard => 9,
            EngineError::InvalidMaxConfigurations => 10,
            EngineError::NegativeUncertainty => 11,
            EngineError::InvalidBound { .. } => 12,
            EngineError::UnknownAssumption(_) => 13,
            EngineError::ConstraintOutsideDomain { .. } => 14,
            EngineError::UnknownParameter(_) => 15,
            EngineError::EmptyConfigurationSet => 16,
            EngineError::ConfigurationSetTooLarge { .. } => 17,
        }
    }

    /// True if `text` contains one of the words banned by R-07.5, as a word
    /// (not as a substring of another word such as "supplied").
    fn has_banned_word(text: &str) -> bool {
        text.split(|c: char| !c.is_ascii_alphabetic())
            .map(str::to_ascii_lowercase)
            .any(|w| {
                w.starts_with("fraud")
                    || w.starts_with("scam")
                    || w.starts_with("hoax")
                    || matches!(
                        w.as_str(),
                        "lie" | "lies" | "lied" | "liar" | "liars" | "lying"
                    )
            })
    }

    #[test]
    fn all_variants_lists_every_variant_once() {
        let mut seen: Vec<usize> = all_variants().iter().map(variant_index).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..18).collect::<Vec<_>>());
    }

    #[test]
    fn no_display_text_uses_a_banned_word() {
        for e in all_variants() {
            let text = e.to_string();
            assert!(!text.is_empty(), "{e:?} has an empty message");
            assert!(!has_banned_word(&text), "banned word in: {text}");
        }
    }

    #[test]
    fn banned_word_check_detects_words_not_substrings() {
        assert!(has_banned_word("this is a Hoax"));
        assert!(has_banned_word("a lie."));
        assert!(has_banned_word("fraudulent claim"));
        assert!(!has_banned_word("value supplied earlier"));
    }

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
