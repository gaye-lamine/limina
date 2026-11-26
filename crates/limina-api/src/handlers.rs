//! HTTP handlers.

use axum::body::Bytes;
use axum::extract::State;
use axum::Json;
use limina_core::decide;
use limina_dsl::{DeclarativeLimit, EqualityLimit, EqualityOutcome};
use serde_json::{json, Value};

use crate::dto::{EvaluateRequest, EvaluateResponse, LimitsResponse};
use crate::error::ApiError;
use crate::{convert, qualified, registry, response, AppState};
use limina_extract::matcher::{CONVERSION_PROCESS, NOT_A_HEAT_ENGINE_CYCLE};

/// `GET /healthz`: liveness probe.
pub async fn healthz() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

/// `GET /v1/limits`: the served limits, then the authored drafts.
pub async fn limits(State(state): State<AppState>) -> Json<LimitsResponse> {
    let mut list = registry::list();
    list.limits.extend(state.authored_limits().iter().map(|l| {
        let mut dto = registry::describe_authored(l);
        dto.agreement = state.agreement(&dto.id);
        dto
    }));
    Json(list)
}

/// `GET /v1/capabilities`: whether extraction is available.
pub async fn capabilities(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"extraction": state.extraction_available()}))
}

/// `POST /v1/evaluate`: runs the deterministic engine on a claim. It never
/// calls a language model.
///
/// The body is parsed here, not by the `Json` extractor, so that every
/// malformed body gives the same 400 `InvalidJson` error.
pub async fn evaluate(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<EvaluateResponse>, ApiError> {
    let request: EvaluateRequest =
        serde_json::from_slice(&body).map_err(|e| ApiError::InvalidJson(e.to_string()))?;
    if registry::find_limit(&request.limit_id).is_none() {
        if let Some(draft) = state.authored(&request.limit_id) {
            return Ok(Json(evaluate_draft(&draft, &request)?));
        }
    }
    Ok(Json(evaluate_request(&request)?))
}

/// The evaluation of an authored limit, worded as a draft. The law gets the
/// assumption `draft_formula`, left open whatever the request says, so the
/// engine answers CONDITIONAL for a claim beyond the AI-written bound: an
/// unvalidated formula never yields IMPOSSIBLE (product rule: when in
/// doubt, never IMPOSSIBLE). The one case the open option cannot cover, a
/// negative claim against a minimum (a bound is never below 0), is refused
/// with no verdict.
pub fn evaluate_draft(
    limit: &DeclarativeLimit,
    request: &EvaluateRequest,
) -> Result<EvaluateResponse, ApiError> {
    let law = qualified::draft(limit);
    let mut request = request.clone();
    request.constraints.remove(qualified::DRAFT_FORMULA);
    let input = convert::decision_input(&law, &request)?;
    let decision = decide(&law, &input)?;
    let mut response = response::build_draft(&law, &input, &decision)?;
    if response.verdict == "IMPOSSIBLE" {
        return Err(ApiError::DraftNotApplied(
            "a negative value cannot be checked against a minimum written by AI".to_string(),
        ));
    }
    if qualified::only_open_option(&response, law.key(), law.open()) {
        response.wording = qualified::draft_wording(&limit.definition().id);
    }
    Ok(response)
}

/// The evaluation of an equality law: both bounds go through the engine and
/// the verdicts are combined; a failed validity condition is the typed error
/// `OutsideModelDomain`, not a verdict.
pub fn evaluate_equality(
    law: &EqualityLimit,
    request: &EvaluateRequest,
) -> Result<EvaluateResponse, ApiError> {
    let input = convert::decision_input(law.upper(), request)?;
    match law.evaluate(&input)? {
        EqualityOutcome::Decided(decision) => Ok(response::build_equality(law, &input, &decision)?),
        EqualityOutcome::OutsideModelDomain { condition } => Err(ApiError::OutsideModelDomain {
            limit_id: law.id().to_string(),
            condition,
        }),
    }
}

/// The evaluation itself, shared by `POST /v1/evaluate` and `POST /v1/analyze`.
pub fn evaluate_request(request: &EvaluateRequest) -> Result<EvaluateResponse, ApiError> {
    if let Some(law) = registry::find_equality(&request.limit_id) {
        return evaluate_equality(&law, request);
    }
    let limit = registry::find_limit(&request.limit_id)
        .ok_or_else(|| ApiError::UnknownLimit(request.limit_id.clone()))?;
    let input = convert::decision_input(limit.as_ref(), request)?;
    let decision = decide(limit.as_ref(), &input)?;
    let mut response = response::build(limit.as_ref(), &input, &decision)?;
    if request.limit_id == "carnot"
        && qualified::only_open_option(&response, CONVERSION_PROCESS, NOT_A_HEAT_ENGINE_CYCLE)
    {
        response.wording = qualified::carnot_wording(&request.limit_id);
    }
    Ok(response)
}
