//! `POST /v1/limits/author` (a draft limit written from a sentence) and
//! `POST /v1/solve` (the value of one parameter at which the bound equals
//! the claim). Spec limit-author.
//!
//! The language model only writes the definition; the DSL checks it and the
//! engine computes every verdict. Authored limits live in memory only.

use std::collections::{BTreeMap, BTreeSet};

use axum::body::Bytes;
use axum::extract::State;
use axum::Json;
use limina_core::{AssumptionId, ConstraintsMap, EngineError, OptionId, Quantity, Verdict};
use limina_dsl::report::{ConditionReport, FormulaReport};
use limina_dsl::{
    solve_for, CheckReport, DeclarativeLimit, DimensionCheck, LimitDefinition, Registry, Solution,
    SolveError, SolveRequest, Status,
};
use limina_extract::AuthorError;
use serde::{Deserialize, Serialize};

use crate::dto::{ClaimDto, ParameterDto};
use crate::error::ApiError;
use crate::AppState;

/// Longest sentence accepted by `POST /v1/limits/author`, in characters.
pub const MAX_AUTHOR_TEXT_CHARS: usize = 2000;

/// Body of `POST /v1/limits/author`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorRequest {
    /// The sentence that states the limit.
    pub text: String,
}

/// Body of a successful `POST /v1/limits/author`.
#[derive(Debug, Clone, Serialize)]
pub struct AuthorResponse {
    /// Id under which the draft is served.
    pub limit_id: String,
    /// Always "draft".
    pub status: &'static str,
    /// The definition as checked and registered.
    pub definition: LimitDefinition,
    /// Model answers used, the first included.
    pub attempts: usize,
    /// Every error sent back to the model, in order.
    pub errors: Vec<String>,
    /// Fields the code set in place of the model's values.
    pub overrides: Vec<String>,
    /// The DSL checks of the registered definition.
    pub report: ReportDto,
}

/// The DSL check report.
#[derive(Debug, Clone, Serialize)]
pub struct ReportDto {
    /// Limit id.
    pub limit_id: String,
    /// Status as written.
    pub status: Status,
    /// True only for a limit validated by a person.
    pub validated: bool,
    /// "passed", or the refusal message.
    pub dimension: String,
    /// Checks per configuration.
    pub formulas: Vec<FormulaReportDto>,
    /// Validity conditions on the grid.
    pub validity: Vec<ConditionReportDto>,
}

/// Checks of the bound under one configuration.
#[derive(Debug, Clone, Serialize)]
pub struct FormulaReportDto {
    /// "assumption = option" pairs.
    pub configuration: String,
    /// Formula used; None when not applicable.
    pub formula: Option<String>,
    /// Parameter name to its trend ("Increasing", "Decreasing",
    /// "NonMonotone", "Undetermined").
    pub monotonicity: BTreeMap<String, String>,
    /// Grid points evaluated.
    pub samples: usize,
    /// Bound finite on the grid; None without a grid.
    pub finite: Option<bool>,
    /// Bound non-negative on the grid; None without a grid.
    pub non_negative: Option<bool>,
}

/// One validity condition on the grid.
#[derive(Debug, Clone, Serialize)]
pub struct ConditionReportDto {
    /// The condition.
    pub condition: String,
    /// Points where it held.
    pub held: usize,
    /// Points where it failed.
    pub failed: usize,
}

/// Body of `POST /v1/solve`. Values are in the given units; `range` is in SI
/// units (kelvin for temperatures).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolveRequestDto {
    /// A DSL limit id: built-in (`betz`, `carnot`, `cop`) or authored.
    pub limit_id: String,
    /// The claim.
    pub claim: ClaimDto,
    /// Assumption id to the allowed option ids.
    #[serde(default)]
    pub constraints: BTreeMap<String, Vec<String>>,
    /// Every parameter other than the free one, fixed.
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterDto>,
    /// The parameter to solve for.
    pub free_parameter: String,
    /// Search range; absent uses the plausible domain, else the test range.
    #[serde(default)]
    pub range: Option<[f64; 2]>,
}

/// Body of a successful `POST /v1/solve`.
#[derive(Debug, Clone, Serialize)]
pub struct SolveResponse {
    /// The limit.
    pub limit_id: String,
    /// The free parameter.
    pub free_parameter: String,
    /// Value at which the bound equals the claim, in `unit`.
    pub threshold: f64,
    /// "at least" or "at most".
    pub direction: &'static str,
    /// Unit of the threshold (SI; kelvin for temperatures).
    pub unit: String,
    /// Search range used.
    pub range: [f64; 2],
    /// Trend of the bound in the free parameter.
    pub monotonicity: String,
    /// Point just below the threshold.
    pub below: f64,
    /// Point just above the threshold.
    pub above: f64,
    /// Engine verdict at `below`.
    pub verdict_below: &'static str,
    /// Engine verdict at `above`.
    pub verdict_above: &'static str,
    /// The answer as a sentence.
    pub sentence: String,
}

/// `POST /v1/limits/author`.
pub async fn author(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<AuthorResponse>, ApiError> {
    let request: AuthorRequest =
        serde_json::from_slice(&body).map_err(|e| ApiError::InvalidJson(e.to_string()))?;
    if request.text.chars().count() > MAX_AUTHOR_TEXT_CHARS {
        return Err(ApiError::TextTooLong {
            max: MAX_AUTHOR_TEXT_CHARS,
        });
    }
    if request.text.trim().is_empty() {
        return Err(ApiError::EmptyText);
    }
    let extractor = state.extractor().ok_or(ApiError::ExtractionUnavailable)?;
    let authored = extractor
        .author_limit(&request.text)
        .await
        .map_err(author_error)?;
    let response = AuthorResponse {
        limit_id: authored.definition.id.clone(),
        status: "draft",
        definition: authored.definition,
        attempts: authored.attempts,
        errors: authored.errors,
        overrides: authored.overrides,
        report: report_dto(&authored.report),
    };
    state.register(authored.limit);
    Ok(Json(response))
}

/// `POST /v1/solve`.
pub async fn solve(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<SolveResponse>, ApiError> {
    let request: SolveRequestDto =
        serde_json::from_slice(&body).map_err(|e| ApiError::InvalidJson(e.to_string()))?;
    Ok(Json(solve_with(&state, &request)?))
}

/// The inverse question itself, shared by `POST /v1/solve` and the
/// `limina check` command.
pub fn solve_with(state: &AppState, request: &SolveRequestDto) -> Result<SolveResponse, ApiError> {
    let (limit, draft) = find_dsl_limit(state, &request.limit_id)
        .ok_or_else(|| ApiError::UnknownLimit(request.limit_id.clone()))?;
    let question = solve_request(&limit, request)?;
    let solution = solve_for(&limit, &question).map_err(solve_error)?;
    Ok(solve_response(request, solution, draft))
}

/// The DSL limit with this id, built-in first; true when it is authored.
pub(crate) fn find_dsl_limit(state: &AppState, id: &str) -> Option<(DeclarativeLimit, bool)> {
    let builtin = Registry::builtin()
        .ok()
        .and_then(|r| r.get(id).cloned())
        .map(|l| (l, false));
    builtin.or_else(|| state.authored(id).map(|l| (l, true)))
}

/// A value in SI units: `K` as kelvin, anything else through the unit table.
pub(crate) fn si(value: f64, unit: &str) -> Result<f64, EngineError> {
    if unit == "K" {
        return Ok(value);
    }
    Ok(Quantity::from_unit(value, unit)?.si_value())
}

fn solve_request(
    limit: &DeclarativeLimit,
    request: &SolveRequestDto,
) -> Result<SolveRequest, ApiError> {
    let mut constraints = ConstraintsMap::new();
    for (assumption, options) in &request.constraints {
        let allowed: BTreeSet<OptionId> = options.iter().map(OptionId::new).collect();
        constraints.allow(AssumptionId::new(assumption), allowed);
    }
    let declared = &limit.definition().parameters;
    let mut fixed = BTreeMap::new();
    for (name, p) in &request.parameters {
        if !declared.iter().any(|d| &d.name == name) {
            return Err(EngineError::UnknownParameter(name.clone()).into());
        }
        if name != &request.free_parameter {
            fixed.insert(name.clone(), si(p.value, &p.unit)?);
        }
    }
    let uncertainty = request
        .claim
        .uncertainty
        .map(|u| si(u, &request.claim.unit))
        .transpose()?;
    Ok(SolveRequest {
        claim: si(request.claim.value, &request.claim.unit)?,
        uncertainty,
        constraints,
        fixed,
        free: request.free_parameter.clone(),
        range: request.range.map(|[lo, hi]| (lo, hi)),
    })
}

pub(crate) fn verdict_name(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Impossible => "IMPOSSIBLE",
        Verdict::Conditional { .. } => "CONDITIONAL",
        Verdict::Compatible => "COMPATIBLE",
        Verdict::InsufficientData { .. } => "INSUFFICIENT_DATA",
    }
}

fn solve_response(request: &SolveRequestDto, s: Solution, draft: bool) -> SolveResponse {
    let sentence = if draft {
        s.sentence.replacen("with limit ", "with draft limit ", 1)
    } else {
        s.sentence
    };
    SolveResponse {
        limit_id: request.limit_id.clone(),
        free_parameter: request.free_parameter.clone(),
        threshold: s.threshold,
        direction: s.direction.as_str(),
        unit: s.unit,
        range: [s.range.0, s.range.1],
        monotonicity: format!("{:?}", s.monotonicity),
        below: s.below,
        above: s.above,
        verdict_below: verdict_name(&s.verdict_below),
        verdict_above: verdict_name(&s.verdict_above),
        sentence,
    }
}

fn author_error(e: AuthorError) -> ApiError {
    match e {
        AuthorError::NotAuthored {
            attempts,
            last_error,
            ..
        } => ApiError::LimitNotAuthored {
            attempts,
            last_error,
        },
        AuthorError::Extract(e) => e.into(),
    }
}

fn solve_error(e: SolveError) -> ApiError {
    match e {
        SolveError::UnknownFreeParameter(p) => ApiError::UnknownFreeParameter(p),
        SolveError::MissingFixedParameter(p) => ApiError::MissingFixedParameter(p),
        SolveError::NoSearchRange(p) => ApiError::NoSearchRange(p),
        SolveError::NotMonotone {
            parameter,
            monotonicity,
        } => ApiError::NotMonotone {
            parameter,
            monotonicity: format!("{monotonicity:?}"),
        },
        SolveError::NoThresholdInRange { parameter, lo, hi } => {
            ApiError::NoThresholdInRange { parameter, lo, hi }
        }
        SolveError::Engine(e) => ApiError::Engine(e),
    }
}

fn formula_dto(f: &FormulaReport) -> FormulaReportDto {
    FormulaReportDto {
        configuration: f.configuration.clone(),
        formula: f.formula.clone(),
        monotonicity: f
            .trends
            .iter()
            .map(|t| (t.parameter.clone(), format!("{:?}", t.monotonicity)))
            .collect(),
        samples: f.samples,
        finite: f.finite,
        non_negative: f.non_negative,
    }
}

fn condition_dto(c: &ConditionReport) -> ConditionReportDto {
    ConditionReportDto {
        condition: c.condition.clone(),
        held: c.held,
        failed: c.failed,
    }
}

/// The check report as JSON.
pub fn report_dto(r: &CheckReport) -> ReportDto {
    ReportDto {
        limit_id: r.limit_id.clone(),
        status: r.status,
        validated: r.validated,
        dimension: match &r.dimension {
            DimensionCheck::Passed => "passed".to_string(),
            DimensionCheck::Failed(e) => e.to_string(),
        },
        formulas: r.formulas.iter().map(formula_dto).collect(),
        validity: r.validity.iter().map(condition_dto).collect(),
    }
}
