//! The limits served by the API.

use limina_core::{
    units_for_dimension, Betz, Carnot, Cop, Dimension, Direction, Limit, MeasureKind,
};

use crate::dto::{AssumptionDto, LimitDto, LimitsResponse, ParameterSpecDto};
use crate::labels::{assumption_label, option_label, parameter_label};
use limina_dsl::{DeclarativeLimit, EqualityLimit, LimitDefinition, Registry};
use limina_extract::{Catalog, CatalogAssumption, CatalogLimit, CatalogOption, CatalogParameter};

/// Looks up a served limit by id.
pub fn find_limit(id: &str) -> Option<Box<dyn Limit>> {
    match id {
        "betz" => Some(Box::new(Betz::new())),
        "carnot" => Some(Box::new(Carnot::new())),
        "cop" => Some(Box::new(Cop::new())),
        _ => None,
    }
}

/// Looks up a built-in equality law by id.
pub fn find_equality(id: &str) -> Option<EqualityLimit> {
    Registry::builtin().ok()?.equality(id).cloned()
}

/// Every served limit, in listing order.
fn all_limits() -> Vec<Box<dyn Limit>> {
    vec![
        Box::new(Betz::new()),
        Box::new(Carnot::new()),
        Box::new(Cop::new()),
    ]
}

/// Display strings of a served limit. The core carries no display text.
struct Display {
    title: &'static str,
    formula: &'static str,
    statement: &'static str,
}

/// Betz: actuator disk, incompressible, unconfined flow.
const BETZ_DISPLAY: Display = Display {
    title: "Betz limit",
    formula: "Cp <= 16/27",
    statement: "An ideal rotor in unconfined, incompressible flow cannot turn more than 16/27 \
                of the wind's kinetic power through its reference area into shaft power.",
};

/// Carnot: ideal reversible cycle between two reservoirs.
const CARNOT_DISPLAY: Display = Display {
    title: "Carnot limit",
    formula: "efficiency <= 1 - Tc/Th",
    statement: "A heat engine working between a hot reservoir at Th and a cold reservoir at Tc, \
                both in kelvin, cannot turn a larger fraction of the heat it takes in into work \
                than 1 - Tc/Th.",
};

/// COP: Carnot bound on a heat pump or refrigerator between two reservoirs.
const COP_DISPLAY: Display = Display {
    title: "Carnot COP limit",
    formula: "COP <= Th/(Th-Tc) (heating), COP <= Tc/(Th-Tc) (cooling)",
    statement: "A heat pump or refrigerator working between a hot reservoir at Th and a cold \
                reservoir at Tc, both in kelvin, cannot reach a coefficient of performance above \
                Th/(Th-Tc) when heating or Tc/(Th-Tc) when cooling.",
};

/// Fallback for a limit without display strings: empty text, never invented.
const NO_DISPLAY: Display = Display {
    title: "",
    formula: "",
    statement: "",
};

/// Display strings of the limit with this id.
fn display_of(id: &str) -> &'static Display {
    match id {
        "betz" => &BETZ_DISPLAY,
        "carnot" => &CARNOT_DISPLAY,
        "cop" => &COP_DISPLAY,
        _ => &NO_DISPLAY,
    }
}

/// Dimension of the unit table's temperature rows (`K`).
const TEMPERATURE: Dimension = Dimension::new([0, 0, 0, 0, 1, 0, 0]);

/// Unit symbols of the core unit table that fit a quantity of `kind`.
fn units_of(kind: MeasureKind) -> Vec<&'static str> {
    match kind {
        MeasureKind::AbsoluteTemperature | MeasureKind::TemperatureDifference => {
            units_for_dimension(TEMPERATURE)
        }
        MeasureKind::Plain(dimension) => units_for_dimension(dimension),
    }
}

/// Describes a limit for `GET /v1/limits`.
pub fn describe(limit: &dyn Limit) -> LimitDto {
    let display = display_of(limit.id());
    LimitDto {
        id: limit.id().to_string(),
        direction: match limit.direction() {
            Direction::Max => "Max",
            Direction::Min => "Min",
        },
        title: display.title.to_string(),
        formula: display.formula.to_string(),
        statement: display.statement.to_string(),
        claim_unit: units_of(limit.bounded_kind())
            .first()
            .map(|u| u.to_string())
            .unwrap_or_default(),
        assumptions: limit
            .assumptions()
            .iter()
            .map(|a| AssumptionDto {
                id: a.id.as_str().to_string(),
                options: a.domain.iter().map(|o| o.id.as_str().to_string()).collect(),
            })
            .collect(),
        parameters: limit
            .parameters()
            .iter()
            .map(|p| ParameterSpecDto {
                id: p.id.as_str().to_string(),
                kind: match p.kind {
                    MeasureKind::AbsoluteTemperature => "AbsoluteTemperature",
                    MeasureKind::TemperatureDifference => "TemperatureDifference",
                    MeasureKind::Plain(_) => "Plain",
                },
                required: p.required,
                units: units_of(p.kind).iter().map(|u| u.to_string()).collect(),
            })
            .collect(),
        assumption_labels: limit
            .assumptions()
            .iter()
            .map(|a| {
                let id = a.id.as_str();
                (id.to_string(), assumption_label(id).to_string())
            })
            .collect(),
        option_labels: limit
            .assumptions()
            .iter()
            .flat_map(|a| a.domain.iter())
            .map(|o| {
                let id = o.id.as_str();
                (id.to_string(), option_label(id).to_string())
            })
            .collect(),
        parameter_labels: limit
            .parameters()
            .iter()
            .map(|p| {
                let id = p.id.as_str();
                (id.to_string(), parameter_label(id).to_string())
            })
            .collect(),
        status: None,
        agreement: None,
        classification: None,
        references: Vec::new(),
        validity_domain: None,
        limitations: Vec::new(),
    }
}

/// Copies the law metadata of a definition into a description.
fn apply_definition(dto: &mut LimitDto, def: &LimitDefinition) {
    dto.classification = def.classification.and_then(|c| {
        serde_json::to_value(c)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
    });
    dto.references = def.references.clone();
    dto.validity_domain = def.validity_domain.clone();
    dto.limitations = def.limitations.clone();
}

/// Describes a limit authored from a text: same fields as the built-in
/// limits, with its own title and formula and `status: "draft"`.
pub fn describe_authored(limit: &DeclarativeLimit) -> LimitDto {
    let def = limit.definition();
    let mut dto = describe(limit);
    dto.title = def.title.clone();
    dto.formula = match &def.formula {
        Some(f) => f.clone(),
        None => limit
            .formulas()
            .iter()
            .map(|f| format!("{}: {}", f.option.as_deref().unwrap_or("all"), f.text))
            .collect::<Vec<_>>()
            .join("; "),
    };
    dto.statement = AUTHORED_STATEMENT.to_string();
    dto.status = Some("draft".to_string());
    apply_definition(&mut dto, def);
    dto
}

/// Statement of every authored limit: it says what it is, nothing more.
pub const AUTHORED_STATEMENT: &str =
    "Draft limit written from a text by a language model and checked by the DSL; not reviewed by a person.";

/// The served limits as the extraction prompt and checks need them, built
/// from the same metadata as `GET /v1/limits`.
pub fn catalog() -> Catalog {
    Catalog {
        limits: list()
            .limits
            .into_iter()
            .map(|l| CatalogLimit {
                id: l.id,
                title: l.title.to_string(),
                formula: l.formula.to_string(),
                claim_unit: l.claim_unit,
                parameters: l
                    .parameters
                    .into_iter()
                    .map(|p| CatalogParameter {
                        name: p.id,
                        units: p.units,
                    })
                    .collect(),
                assumptions: l
                    .assumptions
                    .into_iter()
                    .map(|a| CatalogAssumption {
                        id: a.id,
                        options: a
                            .options
                            .into_iter()
                            .map(|o| CatalogOption {
                                label: option_label(&o).to_string(),
                                id: o,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Content of `GET /v1/limits`.
pub fn list() -> LimitsResponse {
    let definitions = Registry::builtin().ok();
    LimitsResponse {
        limits: all_limits()
            .iter()
            .map(|l| {
                let mut dto = describe(l.as_ref());
                let def = definitions
                    .as_ref()
                    .and_then(|r| r.get(&dto.id))
                    .map(|d| d.definition());
                if let Some(def) = def {
                    apply_definition(&mut dto, def);
                }
                dto
            })
            .collect(),
    }
}
