//! Errors of the DSL: loading, parsing and static checks. Evaluation errors
//! are core `EngineError`s, returned through the `Limit` trait.

/// Seven SI exponents (L, M, T, I, Θ, N, J), for messages.
pub type Exponents = [i8; 7];

/// Every way a definition can be refused.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DslError {
    /// The text of an expression or a condition is not in the grammar.
    #[error("parse error at character {position} of \"{source_text}\": {message}")]
    Parse {
        /// The expression or condition.
        source_text: String,
        /// Character offset of the problem, from 0.
        position: usize,
        /// What was expected.
        message: String,
    },
    /// A name that is neither a parameter, a constant nor a function.
    #[error("unknown symbol \"{name}\" at character {position} of \"{source_text}\"")]
    UnknownSymbol {
        /// The expression or condition.
        source_text: String,
        /// The name.
        name: String,
        /// Character offset of the name, from 0.
        position: usize,
    },
    /// The formula dimension is not the declared bounded dimension.
    #[error("dimension mismatch in {context}: computed {computed:?}, declared {declared:?}")]
    DimensionMismatch {
        /// Which formula.
        context: String,
        /// Dimension computed from the formula.
        computed: Exponents,
        /// Dimension declared by the definition.
        declared: Exponents,
    },
    /// `+`, `-`, `min`, `max` or a comparison between different dimensions.
    #[error("{operation} of {left:?} and {right:?} in \"{source_text}\": the dimensions differ")]
    IncompatibleOperands {
        /// The expression or condition.
        source_text: String,
        /// The operation.
        operation: &'static str,
        /// Dimension of the left operand.
        left: Exponents,
        /// Dimension of the right operand.
        right: Exponents,
    },
    /// `ln` or `exp` of a quantity with a dimension.
    #[error("{function} needs a dimensionless argument in \"{source_text}\", found {found:?}")]
    DimensionlessArgumentRequired {
        /// The expression.
        source_text: String,
        /// The function.
        function: &'static str,
        /// Dimension of the argument.
        found: Exponents,
    },
    /// `sqrt` of a dimension with an odd exponent.
    #[error("sqrt needs even exponents in \"{source_text}\", found {found:?}")]
    OddExponentUnderSqrt {
        /// The expression.
        source_text: String,
        /// Dimension of the argument.
        found: Exponents,
    },
    /// A power or product whose exponent does not fit.
    #[error("dimension exponent out of range in \"{source_text}\"")]
    ExponentOverflow {
        /// The expression.
        source_text: String,
    },
    /// Absolute temperatures and temperature differences mixed against the
    /// core rules (for example absolute plus absolute).
    #[error("{operation} is not allowed between these temperatures in \"{source_text}\"")]
    TemperatureKindMismatch {
        /// The expression or condition.
        source_text: String,
        /// The operation.
        operation: &'static str,
    },
    /// A validity condition the core has no error for.
    #[error("unsupported validity condition \"{condition}\": {reason}")]
    UnsupportedCondition {
        /// The condition.
        condition: String,
        /// Why.
        reason: &'static str,
    },
    /// The definition breaks a rule of the format.
    #[error("invalid definition {limit_id}: {message}")]
    InvalidDefinition {
        /// Id of the definition.
        limit_id: String,
        /// What is wrong.
        message: String,
    },
    /// A definition file could not be read.
    #[error("cannot read {path}: {message}")]
    Io {
        /// File or directory.
        path: String,
        /// System message.
        message: String,
    },
    /// A definition file is not valid JSON of the format.
    #[error("invalid JSON in {path}: {message}")]
    Json {
        /// File.
        path: String,
        /// Parser message.
        message: String,
    },
}
