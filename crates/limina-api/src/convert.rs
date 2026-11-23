//! Conversion of request DTOs into engine inputs.

use std::collections::{BTreeMap, BTreeSet};

use limina_core::{
    AbsoluteTemperature, AssumptionId, Claim, ConstraintsMap, DecisionConfig, DecisionInput,
    EngineError, Limit, Measure, MeasureKind, OptionId, ParameterId, Quantity,
    TemperatureDifference,
};

use crate::dto::EvaluateRequest;

/// Builds a `Measure` of the expected `kind` from a value and a unit symbol.
///
/// Unit `K` gives an absolute temperature or a temperature difference when
/// the limit expects one. Every other symbol goes through the engine's unit
/// table. Nothing is converted between kinds: a mismatch is left to the
/// engine, which returns a typed error.
pub fn measure(kind: MeasureKind, value: f64, unit: &str) -> Result<Measure, EngineError> {
    if unit != "K" {
        return Quantity::from_unit(value, unit).map(Measure::Plain);
    }
    match kind {
        MeasureKind::AbsoluteTemperature => {
            AbsoluteTemperature::from_kelvin(value).map(Measure::AbsoluteTemperature)
        }
        MeasureKind::TemperatureDifference => {
            TemperatureDifference::from_kelvin(value).map(Measure::TemperatureDifference)
        }
        MeasureKind::Plain(_) => Quantity::from_unit(value, unit).map(Measure::Plain),
    }
}

/// Builds the engine input of `request` for `limit`, with default settings.
pub fn decision_input(
    limit: &dyn Limit,
    request: &EvaluateRequest,
) -> Result<DecisionInput, EngineError> {
    let kind = limit.bounded_kind();
    let claim = Claim {
        value: measure(kind, request.claim.value, &request.claim.unit)?,
        uncertainty: request
            .claim
            .uncertainty
            .map(|u| measure(kind.uncertainty_kind(), u, &request.claim.unit))
            .transpose()?,
    };
    let mut constraints = ConstraintsMap::new();
    for (assumption, options) in &request.constraints {
        let allowed: BTreeSet<OptionId> = options.iter().map(OptionId::new).collect();
        constraints.allow(AssumptionId::new(assumption), allowed);
    }
    let mut known_parameters = BTreeMap::new();
    for (name, parameter) in &request.parameters {
        let spec = limit
            .parameters()
            .iter()
            .find(|s| s.id.as_str() == name)
            .ok_or_else(|| EngineError::UnknownParameter(name.clone()))?;
        known_parameters.insert(
            ParameterId::new(name),
            measure(spec.kind, parameter.value, &parameter.unit)?,
        );
    }
    Ok(DecisionInput {
        claim,
        config: DecisionConfig::default(),
        constraints,
        known_parameters,
    })
}
