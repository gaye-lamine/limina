//! The JSON format of a declarative limit.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Upper or lower bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectionName {
    /// The claim may not exceed the bound.
    Max,
    /// The claim may not fall below the bound.
    Min,
}

/// A dimension: seven SI exponents (L, M, T, I, Θ, N, J), or a unit symbol
/// of the core unit table such as "W" or "dimensionless".
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DimensionSpec {
    /// Seven exponents.
    Exponents([i8; 7]),
    /// A unit symbol.
    Unit(String),
}

/// The kind of a temperature parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemperatureKind {
    /// An absolute temperature, in kelvin.
    AbsoluteTemperature,
    /// A temperature difference, in kelvin.
    TemperatureDifference,
}

/// A plausible domain, with where it comes from.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlausibleDefinition {
    /// A closed interval `[lo, hi]` in SI units.
    #[serde(default)]
    pub interval: Option<[f64; 2]>,
    /// A finite set of values in SI units.
    #[serde(default)]
    pub values: Option<Vec<f64>>,
    /// Where the domain comes from.
    pub source: String,
}

/// A parameter of the limit.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterDefinition {
    /// Name used in the formulas.
    pub name: String,
    /// Dimension of a plain parameter.
    #[serde(default)]
    pub dimension: Option<DimensionSpec>,
    /// Kind of a temperature parameter.
    #[serde(default)]
    pub kind: Option<TemperatureKind>,
    /// Allowed unit symbols, all of the parameter dimension.
    pub units: Vec<String>,
    /// Whether the bound needs it.
    #[serde(default = "yes")]
    pub required: bool,
    /// Plausible domain used by the engine when the value is missing.
    #[serde(default)]
    pub plausible: Option<PlausibleDefinition>,
    /// Range used only by the check report, in SI units.
    #[serde(default)]
    pub test_range: Option<[f64; 2]>,
}

fn yes() -> bool {
    true
}

/// An option of an assumption.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionDefinition {
    /// Option id.
    pub id: String,
    /// The bound under this option; overrides the shared formula.
    #[serde(default)]
    pub formula: Option<String>,
    /// A condition the option needs; when it fails the limit is not applicable.
    #[serde(default)]
    pub condition: Option<String>,
}

/// An assumption and its options.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssumptionDefinition {
    /// Assumption id.
    pub id: String,
    /// Its options.
    pub options: Vec<OptionDefinition>,
}

/// Review status. `validated` only counts with a human `validated_by`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Not reviewed.
    #[default]
    Draft,
    /// Reviewed by the person named in `validated_by`.
    Validated,
}

/// How firm the limit is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicStatus {
    /// Follows from established theory.
    Theorem,
    /// Holds under a model's assumptions.
    ModelBound,
    /// Disputed.
    Contested,
}

/// What kind of statement a law is. A model or an empirical law is never
/// presented as fundamental, and a theoretical bound is never presented as
/// technologically achievable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    /// A fundamental law of physics.
    FundamentalLaw,
    /// A statement proved from other accepted statements.
    Theorem,
    /// A relation that describes a class of materials or systems.
    ConstitutiveRelation,
    /// A bound that holds in theory under stated assumptions.
    TheoreticalBound,
    /// A simplified model, valid only in its stated domain.
    Model,
    /// A relation fitted to observations.
    EmpiricalLaw,
}

/// A declarative limit.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LimitDefinition {
    /// Limit id.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Upper or lower bound.
    pub direction: DirectionName,
    /// Dimension of the bounded quantity.
    pub bounded_dimension: DimensionSpec,
    /// Parameters.
    #[serde(default)]
    pub parameters: Vec<ParameterDefinition>,
    /// Assumptions.
    #[serde(default)]
    pub assumptions: Vec<AssumptionDefinition>,
    /// Shared formula; options may override it.
    #[serde(default)]
    pub formula: Option<String>,
    /// Validity conditions, all required.
    #[serde(default)]
    pub validity: Vec<String>,
    /// References.
    #[serde(default)]
    pub references: Vec<String>,
    /// Review status, `draft` by default.
    #[serde(default)]
    pub status: Status,
    /// The person who validated the definition.
    #[serde(default)]
    pub validated_by: Option<String>,
    /// How firm the limit is.
    pub epistemic_status: EpistemicStatus,
    /// What kind of statement the law is; absent in older definitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<Classification>,
    /// Where the law applies, in plain text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validity_domain: Option<String>,
    /// Limits of application, in plain text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
    /// Meaning of each variable name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variables: BTreeMap<String, String>,
}

impl LimitDefinition {
    /// True when a reference is present and none is a placeholder.
    pub fn has_real_reference(&self) -> bool {
        !self.references.is_empty() && self.references.iter().all(|r| !r.starts_with("TODO"))
    }

    /// Validated only when the status says so and a person is named.
    pub fn is_validated(&self) -> bool {
        self.status == Status::Validated
            && self
                .validated_by
                .as_deref()
                .is_some_and(|v| !v.trim().is_empty())
    }
}
