//! `POST /v1/analyze`: read the claims out of a text, check them, then
//! evaluate each with the same code as `POST /v1/evaluate`.

use std::collections::BTreeMap;

use axum::body::Bytes;
use axum::extract::State;
use axum::Json;
use limina_core::EngineError;
use limina_extract::{CheckedClaim, CheckedParameter, Span};

use crate::dto::{
    AnalyzeRequest, AnalyzeResponse, AnalyzedClaimDto, AnalyzedParameterDto, ClaimDto,
    EvaluateRequest, EvidenceDto, ExtractionInfoDto, ParameterDto,
};
use crate::error::ApiError;
use crate::handlers::evaluate_request;
use crate::AppState;

/// Longest text accepted, in characters.
pub const MAX_TEXT_CHARS: usize = 5000;
/// Reason given when the engine cannot apply the limit without a stated
/// assumption (Betz without a reference area).
pub const REFERENCE_AREA_NOT_STATED: &str = "reference area not stated";

/// Status of a claim that went to the engine.
pub const EVALUATED: &str = "evaluated";
/// Status of a claim that needs more input before the engine can decide.
pub const NEEDS_INPUT: &str = "needs_input";

/// Status of a claim whose law does not apply to the given parameters: a
/// validity condition of the law fails. No verdict is given.
pub const OUTSIDE_MODEL_DOMAIN: &str = "outside_model_domain";

/// `POST /v1/analyze`.
pub async fn analyze(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<AnalyzeResponse>, ApiError> {
    let request: AnalyzeRequest =
        serde_json::from_slice(&body).map_err(|e| ApiError::InvalidJson(e.to_string()))?;
    // One deadline for the whole request: the model chains, retries and
    // law authoring can otherwise add up to many minutes.
    let deadline = state.analyze_timeout();
    match tokio::time::timeout(deadline, analyze_text(&state, &request.text)).await {
        Ok(result) => Ok(Json(result?)),
        Err(_) => Err(ApiError::AnalysisTimeout {
            secs: deadline.as_secs().max(1),
        }),
    }
}

/// The analysis itself, shared by `POST /v1/analyze` and the `limina check`
/// command: length and blank checks, extraction, deterministic checks, then
/// each claim evaluated by the engine.
pub async fn analyze_text(state: &AppState, text: &str) -> Result<AnalyzeResponse, ApiError> {
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(ApiError::TextTooLong {
            max: MAX_TEXT_CHARS,
        });
    }
    if text.trim().is_empty() {
        return Err(ApiError::EmptyText);
    }
    let extractor = state.extractor().ok_or(ApiError::ExtractionUnavailable)?;
    let extraction = extractor.extract(text).await?;
    let mut claims: Vec<AnalyzedClaimDto> = extraction.claims.iter().map(analyze_claim).collect();
    let route = limina_extract::Route {
        model: extraction.model.clone(),
        skipped: extraction.skipped.clone(),
    };
    crate::pipeline::complete_claims(state, text, &mut claims, extraction.cached, &route).await;
    Ok(AnalyzeResponse {
        claims,
        missing: extraction.missing,
        extraction: ExtractionInfoDto {
            model: extraction.model,
            cached: extraction.cached,
            ms: extraction.elapsed_ms,
        },
    })
}

fn evidence(span: &Option<Span>) -> Option<EvidenceDto> {
    span.as_ref().map(|s| EvidenceDto {
        text: s.text.clone(),
        start: s.start,
        end: s.end,
    })
}

fn parameter(p: &CheckedParameter) -> AnalyzedParameterDto {
    AnalyzedParameterDto {
        value: p.value,
        raw_value: p.raw_value,
        unit: p.unit.clone(),
        evidence: evidence(&p.evidence),
        flags: p.flags.clone(),
    }
}

/// The request for the engine, from checked values only; None when the claim
/// cannot be evaluated.
fn engine_request(claim: &CheckedClaim) -> Option<EvaluateRequest> {
    if !claim.evaluable() {
        return None;
    }
    let (limit_id, value) = (claim.limit_id.clone()?, claim.value?);
    Some(EvaluateRequest {
        limit_id,
        claim: ClaimDto {
            value,
            unit: claim.unit.clone(),
            uncertainty: None,
        },
        constraints: claim
            .constraints
            .iter()
            .map(|(a, o)| (a.clone(), vec![o.clone()]))
            .collect(),
        parameters: claim
            .parameters
            .iter()
            .filter_map(|(name, p)| {
                p.value.map(|value| {
                    (
                        name.clone(),
                        ParameterDto {
                            value,
                            unit: p.unit.clone(),
                        },
                    )
                })
            })
            .collect(),
    })
}

/// One claim: its checked fields, then the engine result or the reason it
/// needs input.
pub fn analyze_claim(claim: &CheckedClaim) -> AnalyzedClaimDto {
    let mut dto = AnalyzedClaimDto {
        limit_id: claim.limit_id.clone(),
        quantity: claim.quantity.clone(),
        value: claim.value,
        unit: claim.unit.clone(),
        raw_value: claim.raw_value,
        raw_unit: claim.raw_unit.clone(),
        evidence: evidence(&claim.evidence),
        constraints: claim.constraints.clone(),
        parameters: claim
            .parameters
            .iter()
            .map(|(k, p)| (k.clone(), parameter(p)))
            .collect::<BTreeMap<_, _>>(),
        notes: claim.notes.clone(),
        flags: claim.flags.clone(),
        status: NEEDS_INPUT,
        result: None,
        reason: None,
        code: None,
        trace: Vec::new(),
        law: None,
    };
    let Some(request) = engine_request(claim) else {
        dto.reason = claim.not_evaluable_reason().map(str::to_string);
        return dto;
    };
    match evaluate_request(&request) {
        Ok(result) => {
            dto.status = EVALUATED;
            dto.result = Some(result);
        }
        Err(e) => refuse(&mut dto, &e),
    }
    dto
}

/// Records why the engine gave no verdict: `outside_model_domain` when a
/// validity condition of the law fails, `needs_input` otherwise.
pub fn refuse(dto: &mut AnalyzedClaimDto, e: &ApiError) {
    if matches!(e, ApiError::OutsideModelDomain { .. }) {
        dto.status = OUTSIDE_MODEL_DOMAIN;
    }
    dto.reason = Some(match e {
        ApiError::Engine(EngineError::NotApplicable { .. }) => {
            REFERENCE_AREA_NOT_STATED.to_string()
        }
        other => other.to_string(),
    });
    dto.code = Some(e.code());
}
