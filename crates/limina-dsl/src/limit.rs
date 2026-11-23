//! A declarative limit, compiled from its definition and run through the
//! core `Limit` trait.

use std::collections::{BTreeMap, BTreeSet};

use limina_core::{
    units_for_dimension, Assumption, AssumptionId, AssumptionOption, Configuration, Dimension,
    Direction, DomainRange, EngineError, Interval, Limit, MeasureKind, OptionId,
    ParameterEnclosure, ParameterId, ParameterSpec, PlausibleDomain, Quantity,
};

use crate::analysis::{check_condition, check_formula, is_temperature_order, Env, Ty, TEMPERATURE};
use crate::definition::{
    DimensionSpec, DirectionName, LimitDefinition, ParameterDefinition, Status, TemperatureKind,
};
use crate::error::DslError;
use crate::expr::{parse_condition, parse_expression, Condition, Constant, Expr};
use crate::ivops::{eval, holds, Values};

/// A formula with the option it belongs to (None for the shared one).
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledFormula {
    /// Option id, for a per-option formula.
    pub option: Option<String>,
    /// The text.
    pub text: String,
    /// The expression.
    pub expr: Expr,
}

/// A limit defined in the DSL. Every formula and condition was checked for
/// dimensions when it was built.
#[derive(Debug, Clone)]
pub struct DeclarativeLimit {
    definition: LimitDefinition,
    direction: Direction,
    bounded: Dimension,
    assumptions: Vec<Assumption>,
    parameters: Vec<ParameterSpec>,
    names: Vec<String>,
    shared: Option<CompiledFormula>,
    formula_assumption: Option<String>,
    option_formulas: BTreeMap<String, CompiledFormula>,
    option_conditions: Vec<(String, String, Condition)>,
    validity: Vec<Condition>,
}

const FUNCTIONS: [&str; 6] = ["sqrt", "ln", "exp", "abs", "min", "max"];

fn invalid(def: &LimitDefinition, message: impl Into<String>) -> DslError {
    DslError::InvalidDefinition {
        limit_id: def.id.clone(),
        message: message.into(),
    }
}

/// The dimension of a spec; a unit must be in the core unit table.
fn resolve(def: &LimitDefinition, spec: &DimensionSpec) -> Result<Dimension, DslError> {
    match spec {
        DimensionSpec::Exponents(e) => Ok(Dimension::new(*e)),
        DimensionSpec::Unit(u) if u == "K" => Err(invalid(
            def,
            "\"K\" is ambiguous: give a parameter kind absolute_temperature or temperature_difference",
        )),
        DimensionSpec::Unit(u) => Quantity::from_unit(1.0, u)
            .map(|q| q.dimension())
            .map_err(|_| invalid(def, format!("unknown unit \"{u}\""))),
    }
}

fn parameter(
    def: &LimitDefinition,
    p: &ParameterDefinition,
) -> Result<(Ty, ParameterSpec), DslError> {
    if FUNCTIONS.contains(&p.name.as_str()) || Constant::from_name(&p.name).is_some() {
        return Err(invalid(
            def,
            format!("parameter name \"{}\" is reserved", p.name),
        ));
    }
    let (ty, kind) = match (&p.dimension, p.kind) {
        (Some(d), None) => {
            let dim = resolve(def, d)?;
            if dim.exponents() == TEMPERATURE {
                return Err(invalid(
                    def,
                    format!("parameter {} is a temperature: give its kind", p.name),
                ));
            }
            (Ty::Plain(dim), MeasureKind::Plain(dim))
        }
        (None, Some(TemperatureKind::AbsoluteTemperature)) => {
            (Ty::Absolute, MeasureKind::AbsoluteTemperature)
        }
        (None, Some(TemperatureKind::TemperatureDifference)) => {
            (Ty::Difference, MeasureKind::TemperatureDifference)
        }
        _ => {
            return Err(invalid(
                def,
                format!(
                    "parameter {} needs exactly one of dimension or kind",
                    p.name
                ),
            ))
        }
    };
    let allowed = units_for_dimension(Dimension::new(ty.exponents()));
    if p.units.is_empty() || p.units.iter().any(|u| !allowed.contains(&u.as_str())) {
        return Err(invalid(
            def,
            format!(
                "units of {} must be core units of its dimension: {allowed:?}",
                p.name
            ),
        ));
    }
    let plausible = match &p.plausible {
        None => None,
        Some(pd) => {
            if pd.source.trim().is_empty() {
                return Err(invalid(
                    def,
                    format!("plausible domain of {} needs a source", p.name),
                ));
            }
            let range = match (&pd.interval, &pd.values) {
                (Some([lo, hi]), None) => {
                    DomainRange::Interval(Interval::new(*lo, *hi).map_err(|e| {
                        invalid(def, format!("plausible domain of {}: {e}", p.name))
                    })?)
                }
                (None, Some(v)) if !v.is_empty() && v.iter().all(|x| x.is_finite()) => {
                    DomainRange::Finite(v.clone())
                }
                _ => {
                    return Err(invalid(
                        def,
                        format!("plausible domain of {} needs an interval or values", p.name),
                    ))
                }
            };
            Some(PlausibleDomain {
                range,
                source: pd.source.clone(),
            })
        }
    };
    if let Some([lo, hi]) = p.test_range {
        if !(lo.is_finite() && hi.is_finite() && lo < hi) {
            return Err(invalid(
                def,
                format!("test_range of {} must be finite with lo < hi", p.name),
            ));
        }
    }
    Ok((
        ty,
        ParameterSpec {
            id: ParameterId::new(p.name.clone()),
            kind,
            required: p.required,
            plausible,
        },
    ))
}

fn unique<'a>(
    def: &LimitDefinition,
    ids: impl Iterator<Item = &'a str>,
    what: &str,
) -> Result<(), DslError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.is_empty() || !seen.insert(id) {
            return Err(invalid(
                def,
                format!("{what} ids must be non-empty and unique"),
            ));
        }
    }
    Ok(())
}

impl DeclarativeLimit {
    /// Compiles a definition: parses and checks every formula and condition.
    pub fn compile(definition: LimitDefinition) -> Result<DeclarativeLimit, DslError> {
        let def = &definition;
        if def.id.trim().is_empty() {
            return Err(invalid(def, "the id is empty"));
        }
        if def.status == Status::Validated && !def.is_validated() {
            return Err(invalid(
                def,
                "status validated needs a person in validated_by",
            ));
        }
        let bounded = resolve(def, &def.bounded_dimension)?;
        if bounded.exponents() == TEMPERATURE {
            return Err(invalid(def, "a temperature bound is not supported"));
        }
        unique(
            def,
            def.parameters.iter().map(|p| p.name.as_str()),
            "parameter",
        )?;
        unique(
            def,
            def.assumptions.iter().map(|a| a.id.as_str()),
            "assumption",
        )?;
        let mut env = Env::new();
        let mut parameters = Vec::new();
        for p in &def.parameters {
            let (ty, spec) = parameter(def, p)?;
            env.insert(p.name.clone(), ty);
            parameters.push(spec);
        }
        let names: Vec<String> = def.parameters.iter().map(|p| p.name.clone()).collect();
        let formula = |text: &str, option: Option<&str>| -> Result<CompiledFormula, DslError> {
            let expr = parse_expression(text, &names)?;
            let context = option.map_or_else(
                || "formula".to_string(),
                |o| format!("formula of option {o}"),
            );
            check_formula(&expr, &env, text, bounded, &context)?;
            Ok(CompiledFormula {
                option: option.map(str::to_string),
                text: text.to_string(),
                expr,
            })
        };
        let shared = def
            .formula
            .as_deref()
            .map(|f| formula(f, None))
            .transpose()?;
        let mut formula_assumption = None;
        let mut option_formulas = BTreeMap::new();
        let mut option_conditions = Vec::new();
        let mut assumptions = Vec::new();
        for a in &def.assumptions {
            if a.options.is_empty() {
                return Err(invalid(def, format!("assumption {} has no option", a.id)));
            }
            unique(def, a.options.iter().map(|o| o.id.as_str()), "option")?;
            if a.options.iter().any(|o| o.formula.is_some()) {
                if formula_assumption.is_some() {
                    return Err(invalid(
                        def,
                        "only one assumption may carry per-option formulas",
                    ));
                }
                formula_assumption = Some(a.id.clone());
            }
            for o in &a.options {
                if let Some(f) = &o.formula {
                    option_formulas.insert(o.id.clone(), formula(f, Some(&o.id))?);
                }
                if let Some(c) = &o.condition {
                    let cond = parse_condition(c, &names)?;
                    check_condition(&cond, &env)?;
                    option_conditions.push((a.id.clone(), o.id.clone(), cond));
                }
            }
            assumptions.push(Assumption {
                id: AssumptionId::new(a.id.clone()),
                domain: a
                    .options
                    .iter()
                    .map(|o| AssumptionOption {
                        id: OptionId::new(o.id.clone()),
                        value: None,
                    })
                    .collect(),
            });
        }
        if shared.is_none() && option_formulas.is_empty() {
            return Err(invalid(def, "no formula"));
        }
        let mut validity = Vec::new();
        for c in &def.validity {
            let cond = parse_condition(c, &names)?;
            let types = check_condition(&cond, &env)?;
            if !is_temperature_order(&cond, types) {
                return Err(DslError::UnsupportedCondition {
                    condition: c.clone(),
                    reason: "the core has no error for a failed validity condition other than a temperature order (InvalidTemperatureOrder)",
                });
            }
            validity.push(cond);
        }
        Ok(DeclarativeLimit {
            direction: match def.direction {
                DirectionName::Max => Direction::Max,
                DirectionName::Min => Direction::Min,
            },
            definition,
            bounded,
            assumptions,
            parameters,
            names,
            shared,
            formula_assumption,
            option_formulas,
            option_conditions,
            validity,
        })
    }

    /// Parses and compiles a JSON definition.
    pub fn from_json(json: &str) -> Result<DeclarativeLimit, DslError> {
        let definition: LimitDefinition =
            serde_json::from_str(json).map_err(|e| DslError::Json {
                path: "<text>".to_string(),
                message: e.to_string(),
            })?;
        DeclarativeLimit::compile(definition)
    }

    /// The definition it was built from.
    pub fn definition(&self) -> &LimitDefinition {
        &self.definition
    }

    /// Validated only when the status says so and a person is named.
    pub fn is_validated(&self) -> bool {
        self.definition.is_validated()
    }

    /// Dimension of the bounded quantity.
    pub fn bounded_dimension(&self) -> Dimension {
        self.bounded
    }

    /// Every compiled formula: the shared one, then per option.
    pub fn formulas(&self) -> Vec<&CompiledFormula> {
        self.shared
            .iter()
            .chain(self.option_formulas.values())
            .collect()
    }

    pub(crate) fn validity(&self) -> &[Condition] {
        &self.validity
    }

    pub(crate) fn parameter_names(&self) -> &[String] {
        &self.names
    }

    /// The formula for this configuration: the option's own, else the shared
    /// one; `Err` with the option when there is none (not applicable).
    pub(crate) fn formula_for(
        &self,
        config: &Configuration,
    ) -> Result<&CompiledFormula, EngineError> {
        if let Some(a) = &self.formula_assumption {
            let option = config
                .option(&AssumptionId::new(a.clone()))
                .ok_or_else(|| EngineError::UnknownAssumption(a.clone()))?;
            if let Some(f) = self.option_formulas.get(option.as_str()) {
                return Ok(f);
            }
            return self
                .shared
                .as_ref()
                .ok_or_else(|| EngineError::NotApplicable {
                    limit_id: self.definition.id.clone(),
                    assumption: a.clone(),
                    option: option.as_str().to_string(),
                });
        }
        // Unreachable: compile refuses a definition without any formula.
        self.shared
            .as_ref()
            .ok_or_else(|| EngineError::InvalidBound {
                limit_id: self.definition.id.clone(),
            })
    }

    /// Evaluates a formula on these values, with the validity and option
    /// conditions; used by `bound` and by the check report.
    pub(crate) fn evaluate(
        &self,
        config: &Configuration,
        formula: &Expr,
        values: &Values,
    ) -> Result<Interval, EngineError> {
        for c in &self.validity {
            if !holds(c, values)? {
                return Err(EngineError::InvalidTemperatureOrder {
                    limit_id: self.definition.id.clone(),
                });
            }
        }
        for (assumption, option, c) in &self.option_conditions {
            let selected = config
                .option(&AssumptionId::new(assumption.clone()))
                .map(OptionId::as_str);
            if selected == Some(option.as_str()) && !holds(c, values)? {
                return Err(EngineError::NotApplicable {
                    limit_id: self.definition.id.clone(),
                    assumption: assumption.clone(),
                    option: option.clone(),
                });
            }
        }
        eval(formula, values)
    }
}

impl Limit for DeclarativeLimit {
    fn id(&self) -> &str {
        &self.definition.id
    }

    fn direction(&self) -> Direction {
        self.direction
    }

    fn bounded_kind(&self) -> MeasureKind {
        MeasureKind::Plain(self.bounded)
    }

    fn assumptions(&self) -> &[Assumption] {
        &self.assumptions
    }

    fn parameters(&self) -> &[ParameterSpec] {
        &self.parameters
    }

    /// Order, as in the Rust limits: options (`UnknownAssumption`,
    /// `ConstraintOutsideDomain`), formula (`NotApplicable` when the option
    /// has none), parameters in declared order (`UnknownParameter`),
    /// validity (`InvalidTemperatureOrder`), option conditions
    /// (`NotApplicable`), then the interval value of the formula.
    fn bound(
        &self,
        config: &Configuration,
        params: &ParameterEnclosure,
    ) -> Result<Interval, EngineError> {
        for a in &self.assumptions {
            let option = config
                .option(&a.id)
                .ok_or_else(|| EngineError::UnknownAssumption(a.id.as_str().to_string()))?;
            if !a.domain.iter().any(|o| &o.id == option) {
                return Err(EngineError::ConstraintOutsideDomain {
                    assumption: a.id.as_str().to_string(),
                    option: option.as_str().to_string(),
                });
            }
        }
        let formula = self.formula_for(config)?;
        let mut values = Values::new();
        for name in &self.names {
            let v = params
                .get(&ParameterId::new(name.clone()))
                .ok_or_else(|| EngineError::UnknownParameter(name.clone()))?;
            values.insert(name.clone(), v);
        }
        self.evaluate(config, &formula.expr, &values)
    }
}
