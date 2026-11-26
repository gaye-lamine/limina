//! The automatic pipeline of `POST /v1/analyze` for claims with no
//! built-in limit, the trace of every claim, and the endpoints that serve
//! the explorer: `POST /v1/curve`, `POST /v1/transcribe`, `GET /v1/pipeline`.
//!
//! For a claim with no built-in limit: (i) a model names the established
//! law that bounds it, or answers "no law identified"; (ii) two models
//! write that law in the DSL and the two definitions are compared on 200
//! samples; (iii) a second extraction pass reads the law's parameters from
//! the same text with the same evidence rules; (iv) the claim is evaluated
//! by the engine, only when the two models agree. Every step is recorded.

use std::collections::BTreeMap;

use axum::body::Bytes;
use axum::extract::State;
use axum::Json;
use limina_core::{Direction, DomainRange, Limit};
use limina_dsl::{bound_curve_at, DeclarativeLimit, Registry};
use limina_extract::check::{number_in, LIMIT_NOT_RECOGNIZED};
use limina_extract::{
    transcribe_image_routed, AuthorError, ImageInput, LawAnswer, LawStatus, ModelAttempt, Route,
    TwoModelLaw, WantedParameter,
};

use crate::authoring::verdict_name;
use crate::dto::{
    AnalyzedClaimDto, AnalyzedParameterDto, ClaimDto, ComparisonDto, CurveRequest, CurveResponse,
    EvaluateRequest, EvidenceDto, LawAttemptDto, LawDto, ParameterDto, PipelineInfoDto,
    TraceStepDto, TranscribeRequest, TranscribeResponse,
};
use crate::error::ApiError;
use crate::handlers::{evaluate_draft, evaluate_request};
use crate::{graphs, registry, AppState};

/// Status word of a step that ran and succeeded.
pub const DONE: &str = "done";
/// Status word of a step that ran and failed.
pub const FAILED: &str = "failed";
/// Status word of a step that did not run.
pub const SKIPPED: &str = "skipped";

/// Default number of points of `POST /v1/curve`.
pub const DEFAULT_CURVE_POINTS: usize = 101;

fn step(name: &str, status: &'static str, detail: impl Into<String>) -> TraceStepDto {
    TraceStepDto {
        step: name.to_string(),
        status,
        detail: detail.into(),
    }
}

/// A number for the trace: at most 10 significant digits, so the last-bit
/// noise of a decimal reading ("1.0000000000000001e-24") is not shown.
fn number(v: f64) -> String {
    limina_extract::check::short_number(v)
}

fn verdict_step(claim: &AnalyzedClaimDto) -> TraceStepDto {
    match (&claim.result, &claim.reason) {
        (Some(r), _) => step("evaluation", DONE, format!("engine verdict {}", r.verdict)),
        (None, Some(reason)) => step("evaluation", FAILED, format!("not evaluated: {reason}")),
        (None, None) => step("evaluation", FAILED, "not evaluated"),
    }
}

fn parameters_detail(claim: &AnalyzedClaimDto, declared: &[String]) -> (bool, String) {
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for name in declared {
        match claim.parameters.get(name) {
            Some(p) if p.value.is_some() => found.push(format!(
                "{name} = {} {}",
                number(p.value.unwrap_or(0.0)),
                p.unit
            )),
            Some(p) => missing.push(format!("{name} ({})", p.flags.join(", "))),
            None => missing.push(format!("{name} (not stated in the text)")),
        }
    }
    let mut parts = Vec::new();
    if !found.is_empty() {
        parts.push(format!("found {}", found.join(", ")));
    }
    if !missing.is_empty() {
        parts.push(format!("missing {}", missing.join(", ")));
    }
    if parts.is_empty() {
        parts.push("the limit needs no parameter".to_string());
    }
    (missing.is_empty(), parts.join("; "))
}

fn extraction_step(claim: &AnalyzedClaimDto, cached: bool, route: &Route) -> TraceStepDto {
    let at = claim
        .evidence
        .as_ref()
        .map(|e| format!(", evidence \"{}\"", e.text))
        .unwrap_or_else(|| ", no evidence found in the text".to_string());
    let flags = if claim.flags.is_empty() {
        String::new()
    } else {
        format!("; flags: {}", claim.flags.join(", "))
    };
    step(
        "extraction",
        DONE,
        format!(
            "read {}: {} {}{at}{}{flags}; {}",
            claim.quantity,
            number(claim.raw_value),
            claim.raw_unit,
            if cached { " (cached answer)" } else { "" },
            route.describe()
        ),
    )
}

fn built_in(claim: &AnalyzedClaimDto) -> Option<String> {
    let id = claim.limit_id.clone()?;
    (registry::find_limit(&id).is_some() && !claim.flags.iter().any(|f| f == LIMIT_NOT_RECOGNIZED))
        .then_some(id)
}

/// Adds the trace to every claim, and runs the law pipeline on the claims
/// with no built-in limit when two-model authoring is configured.
pub async fn complete_claims(
    state: &AppState,
    text: &str,
    claims: &mut Vec<AnalyzedClaimDto>,
    cached: bool,
    route: &Route,
) {
    merge_duplicates(claims);
    for claim in claims.iter_mut() {
        let first = extraction_step(claim, cached, route);
        claim.trace.insert(0, first);
        if let Some(id) = built_in(claim) {
            claim
                .trace
                .push(step("limit", DONE, format!("built-in limit {id}")));
            let declared: Vec<String> = registry::find_limit(&id)
                .map(|l| {
                    l.parameters()
                        .iter()
                        .map(|p| p.id.as_str().to_string())
                        .collect()
                })
                .unwrap_or_default();
            let (all, detail) = parameters_detail(claim, &declared);
            claim
                .trace
                .push(step("parameters", if all { DONE } else { FAILED }, detail));
            claim.trace.push(verdict_step(claim));
            claim.law = Some(LawDto {
                source: "built_in",
                limit_id: Some(id),
                sentence: None,
                attempts: Vec::new(),
                comparison: None,
            });
            continue;
        }
        claim.trace.push(step(
            "limit",
            FAILED,
            "no built-in limit matches this claim",
        ));
        law_pipeline(state, text, claim).await;
    }
}

fn overlaps(a: &Option<EvidenceDto>, b: &Option<EvidenceDto>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.start < b.end && b.start < a.end,
        _ => false,
    }
}

/// The same number read at the same place.
fn same_reading(kept: &AnalyzedClaimDto, other: &AnalyzedClaimDto) -> bool {
    kept.raw_value == other.raw_value && overlaps(&kept.evidence, &other.evidence)
}

/// A reading the text does not write (no evidence) whose number and unit
/// are exactly the kept claim's value after its unit conversion: the model
/// wrote "12 km/s" and also "12000 m/s". Both have the same law.
fn converted_copy(kept: &AnalyzedClaimDto, other: &AnalyzedClaimDto) -> bool {
    other.evidence.is_none()
        && other.value.is_none()
        && kept.evidence.is_some()
        && built_in(kept) == built_in(other)
        && kept.unit == other.raw_unit
        && kept.value.is_some_and(|v| {
            (v - other.raw_value).abs() <= 1e-9 * v.abs().max(other.raw_value.abs())
        })
}

/// A claim with no built-in limit that reads the same number at the same
/// place as a claim with a built-in limit is the same claim read twice by
/// the model: it is dropped, and the kept claim says so in its trace.
fn merge_duplicates(claims: &mut Vec<AnalyzedClaimDto>) {
    let mut drop = vec![false; claims.len()];
    let mut merged = vec![0usize; claims.len()];
    let mut as_parameter = vec![0usize; claims.len()];
    for i in 0..claims.len() {
        let own = built_in(&claims[i]);
        // A built-in claim is a twin only of an earlier reading of the same law.
        let twin = (0..claims.len()).find(|&j| {
            j != i
                && !drop[j]
                && built_in(&claims[j]).is_some()
                && (own.is_none() || (j < i && built_in(&claims[j]) == own))
                && (same_reading(&claims[j], &claims[i]) || converted_copy(&claims[j], &claims[i]))
        });
        if let Some(j) = twin {
            drop[i] = true;
            merged[j] += 1;
            continue;
        }
        if own.is_some() {
            continue;
        }
        // A parameter of a built-in claim read again as a claim of its own:
        // same number at the place of that parameter's evidence.
        let host = (0..claims.len()).find(|&j| {
            j != i
                && !drop[j]
                && built_in(&claims[j]).is_some()
                && claims[j].parameters.values().any(|p| {
                    p.raw_value == claims[i].raw_value && overlaps(&p.evidence, &claims[i].evidence)
                })
        });
        if let Some(j) = host {
            drop[i] = true;
            as_parameter[j] += 1;
        }
    }
    let mut k = 0;
    claims.retain(|_| {
        k += 1;
        !drop[k - 1]
    });
    let kept: Vec<(usize, usize)> = (0..merged.len())
        .filter(|i| !drop[*i])
        .map(|i| (merged[i], as_parameter[i]))
        .collect();
    for (claim, (n, p)) in claims.iter_mut().zip(kept) {
        if n > 0 {
            claim.trace.push(step(
                "deduplication",
                DONE,
                format!("{n} other reading(s) of the same number at the same place merged into this claim"),
            ));
        }
        if p > 0 {
            claim.trace.push(step(
                "deduplication",
                DONE,
                format!("{p} reading(s) of one of this claim's parameters as a claim of its own, without a known limit, merged into this claim"),
            ));
        }
    }
}

/// The claim's traced parameters in SI units, for the engine.
fn si_parameters(claim: &AnalyzedClaimDto) -> BTreeMap<String, f64> {
    claim
        .parameters
        .iter()
        .filter_map(|(k, p)| {
            let v = p.value?;
            let unit = p.unit.trim();
            Some((k.clone(), v * si_factor(unit)?))
        })
        .collect()
}

/// A unit factor to SI; 1 for a dimensionless or empty unit.
fn si_factor(unit: &str) -> Option<f64> {
    let unit = unit.trim();
    if unit.is_empty() || unit == "dimensionless" {
        Some(1.0)
    } else {
        limina_core::parse_unit(unit).ok().map(|u| u.factor_to_si)
    }
}

/// The claim's parameters for one candidate definition, in SI units: the
/// traced ones, then each missing one read from the claim's sentence when
/// exactly one number there is in a unit of that parameter.
fn candidate_parameters(
    claim: &AnalyzedClaimDto,
    text: &str,
    definition: &limina_dsl::LimitDefinition,
) -> BTreeMap<String, f64> {
    let mut values = si_parameters(claim);
    let Some(ev) = claim.evidence.as_ref() else {
        return values;
    };
    for p in &definition.parameters {
        if values.contains_key(&p.name) {
            continue;
        }
        let read = limina_extract::check::unique_number_in_units(text, ev.start, ev.end, &p.units);
        if let Some((v, u)) = read {
            if let Some(f) = si_factor(&u) {
                values.insert(p.name.clone(), v * f);
            }
        }
    }
    values
}

fn attempt_dto(a: &ModelAttempt, claim: &AnalyzedClaimDto, text: &str) -> LawAttemptDto {
    match &a.result {
        Ok(x) => {
            let values = candidate_parameters(claim, text, &x.definition);
            let at = limina_dsl::decisive_bound_at(&x.limit, &values);
            LawAttemptDto {
                model: a.model.clone(),
                ok: true,
                attempts: x.attempts,
                errors: x.errors.clone(),
                definition: Some(x.definition.clone()),
                bound_at_claim: at.as_ref().ok().copied(),
                bound_note: at.err(),
            }
        }
        Err(AuthorError::NotAuthored {
            attempts, errors, ..
        }) => LawAttemptDto {
            model: a.model.clone(),
            ok: false,
            attempts: *attempts,
            errors: errors.clone(),
            definition: None,
            bound_at_claim: None,
            bound_note: None,
        },
        Err(e) => LawAttemptDto {
            model: a.model.clone(),
            ok: false,
            attempts: 0,
            errors: vec![e.to_string()],
            definition: None,
            bound_at_claim: None,
            bound_note: None,
        },
    }
}

fn skipped_note(skipped: &[String]) -> String {
    if skipped.is_empty() {
        String::new()
    } else {
        format!("; skipped: {}", skipped.join(", "))
    }
}

fn attempt_step(a: &ModelAttempt) -> TraceStepDto {
    let name = format!("authoring by {}", a.model);
    match &a.result {
        Ok(x) => step(
            &name,
            DONE,
            format!(
                "valid definition after {} attempt(s){}; DSL checks passed; formula {}{}",
                x.attempts,
                if x.errors.is_empty() {
                    String::new()
                } else {
                    format!(" (repaired: {})", x.errors.join(" | "))
                },
                x.definition
                    .formula
                    .clone()
                    .unwrap_or_else(|| "per option".into()),
                skipped_note(&a.skipped)
            ),
        ),
        Err(e) => step(&name, FAILED, e.to_string()),
    }
}

fn comparison_step(law: &TwoModelLaw) -> TraceStepDto {
    match (law.status, &law.comparison) {
        (LawStatus::Agreed, Some(c)) => step(
            "comparison",
            DONE,
            format!(
                "draft_agreed: the two definitions agree on {} samples (largest relative difference {:e})",
                c.samples, c.max_relative_difference
            ),
        ),
        (_, Some(c)) => step(
            "comparison",
            FAILED,
            format!(
                "draft_disputed: {}",
                c.reason.clone().unwrap_or_else(|| "the definitions differ".into())
            ),
        ),
        (LawStatus::Disputed, None) if law.a.result.is_ok() && law.b.result.is_ok() => step(
            "comparison",
            FAILED,
            format!(
                "draft_disputed: both definitions were written by the same model {} (chain B fell back to it); one author only, not compared",
                law.a.model
            ),
        ),
        (LawStatus::NotAuthored, None) => step(
            "comparison",
            SKIPPED,
            "not_authored: neither model wrote a valid definition",
        ),
        (_, None) => step(
            "comparison",
            FAILED,
            "draft_disputed: only one model wrote a valid definition",
        ),
    }
}

fn needs_input(claim: &mut AnalyzedClaimDto, reason: &str) {
    claim.status = crate::analyze::NEEDS_INPUT;
    claim.result = None;
    claim.reason = Some(reason.to_string());
}

/// Steps (i) to (iv) for one claim with no built-in limit.
async fn law_pipeline(state: &AppState, text: &str, claim: &mut AnalyzedClaimDto) {
    let (Some(extractor), Some(second)) = (state.extractor(), state.second()) else {
        claim.trace.push(step(
            "law",
            SKIPPED,
            "two-model law authoring is not configured on this server",
        ));
        return;
    };
    // A claim merged from two readings of one sentence (energy in a mass)
    // is traceable through both readings, recorded in its note.
    let merged = claim
        .notes
        .iter()
        .any(|n| n.starts_with(limina_extract::check::MERGED_QUOTIENT));
    let traced = claim
        .evidence
        .as_ref()
        .is_some_and(|e| merged || number_in(&e.text, claim.raw_value));
    let Some(evidence) = claim.evidence.clone().filter(|_| traced) else {
        claim.trace.push(step(
            "law",
            SKIPPED,
            "claim value not traceable to the text",
        ));
        return;
    };
    // (i) Which law bounds this quantity.
    let question = format!(
        "Quantity: {}. Claimed value: {} {}. Sentence: {}",
        claim.quantity,
        number(claim.raw_value),
        claim.raw_unit,
        evidence.text
    );
    let sentence = match extractor.identify_law_in_library_routed(&question).await {
        Ok((LawAnswer::BuiltIn(id), route)) => {
            built_in_path(extractor, text, claim, &id, &route).await;
            return;
        }
        Ok((LawAnswer::Sentence(s), route)) => {
            claim
                .trace
                .push(step("law", DONE, format!("{s} ({})", route.describe())));
            s
        }
        Ok((LawAnswer::NoLaw, route)) => {
            claim.trace.push(step(
                "law",
                FAILED,
                format!("{} ({})", limina_extract::NO_LAW, route.describe()),
            ));
            claim.law = Some(LawDto {
                source: "no_law_identified",
                limit_id: None,
                sentence: None,
                attempts: Vec::new(),
                comparison: None,
            });
            needs_input(claim, "no established law identified for this quantity");
            return;
        }
        Err(e) => {
            claim.trace.push(step("law", FAILED, e.to_string()));
            needs_input(claim, &format!("the law could not be identified: {e}"));
            return;
        }
    };
    // (ii) Two models write it; the definitions are compared.
    let brief = format!(
        "{sentence}\nThe bounded quantity is: {} (claimed in {}).",
        claim.quantity, claim.raw_unit
    );
    let law = extractor.author_two(second, &brief).await;
    claim.trace.push(attempt_step(&law.a));
    claim.trace.push(attempt_step(&law.b));
    claim.trace.push(comparison_step(&law));
    let agreed = law.agreed_limit().cloned();
    claim.law = Some(LawDto {
        source: law.status.as_str(),
        limit_id: agreed.as_ref().map(|a| a.definition.id.clone()),
        sentence: Some(sentence),
        attempts: vec![
            attempt_dto(&law.a, claim, text),
            attempt_dto(&law.b, claim, text),
        ],
        comparison: law.comparison.as_ref().map(|c| ComparisonDto {
            samples: c.samples,
            max_relative_difference: c.max_relative_difference,
            reason: c.reason.clone(),
        }),
    });
    let Some(authored) = agreed else {
        let reason = match law.status {
            LawStatus::NotAuthored => "no valid definition of the law could be written",
            _ => "the two models wrote different laws; no verdict is given",
        };
        claim
            .trace
            .push(step("parameters", SKIPPED, "no agreed law"));
        claim.trace.push(step(
            "evaluation",
            SKIPPED,
            format!("not evaluated: {reason}"),
        ));
        needs_input(claim, reason);
        return;
    };
    let id = authored.definition.id.clone();
    state.register(authored.limit.clone());
    state.set_agreement(&id, LawStatus::Agreed.as_str());
    claim.limit_id = Some(id.clone());
    // (iii) The law's parameters, from the same text.
    let wanted: Vec<WantedParameter> = authored
        .definition
        .parameters
        .iter()
        .map(|p| WantedParameter {
            name: p.name.clone(),
            units: p.units.clone(),
            meaning: String::new(),
        })
        .collect();
    read_parameters(extractor, text, claim, &wanted).await;
    // (iv) The engine.
    let request = engine_request(claim, id);
    match evaluate_draft(&authored.limit, &request) {
        Ok(result) => {
            claim.status = crate::analyze::EVALUATED;
            claim.reason = None;
            claim.code = None;
            claim.result = Some(result);
        }
        Err(e) => {
            claim.code = Some(e.code());
            needs_input(claim, &e.to_string());
        }
    }
    claim.trace.push(verdict_step(claim));
}

/// Reads the parameters of the law out of the text and records them, with
/// their trace step, on the claim.
async fn read_parameters(
    extractor: &limina_extract::Extractor,
    text: &str,
    claim: &mut AnalyzedClaimDto,
    wanted: &[WantedParameter],
) {
    claim.parameters.clear();
    if wanted.is_empty() {
        claim
            .trace
            .push(step("parameters", SKIPPED, "the law needs no parameter"));
        return;
    }
    match extractor.extract_parameters_routed(text, wanted).await {
        Ok((found, route)) => {
            for f in found {
                claim.parameters.insert(
                    f.name.clone(),
                    AnalyzedParameterDto {
                        value: f.value,
                        raw_value: f.value.unwrap_or(f64::NAN),
                        unit: f.unit,
                        evidence: f.evidence.map(|s| EvidenceDto {
                            text: s.text,
                            start: s.start,
                            end: s.end,
                        }),
                        flags: f.flags,
                    },
                );
            }
            let names: Vec<String> = wanted.iter().map(|w| w.name.clone()).collect();
            let (all, detail) = parameters_detail(claim, &names);
            claim.trace.push(step(
                "parameters",
                if all { DONE } else { FAILED },
                format!("{detail}; {}", route.describe()),
            ));
        }
        Err(e) => claim.trace.push(step(
            "parameters",
            FAILED,
            format!("second extraction pass failed: {e}"),
        )),
    }
}

/// The engine request of a claim whose law was identified by name: the
/// claim value (a percentage becomes a fraction) and the parameters read.
/// No assumption is stated on this path, so none is passed.
fn engine_request(claim: &mut AnalyzedClaimDto, limit_id: String) -> EvaluateRequest {
    let (value, unit) = if claim.raw_unit == "%" {
        claim.notes.push(format!(
            "{} % becomes {}",
            number(claim.raw_value),
            number(claim.raw_value / 100.0)
        ));
        (claim.raw_value / 100.0, "dimensionless".to_string())
    } else {
        (claim.raw_value, claim.raw_unit.clone())
    };
    claim.value = Some(value);
    claim.unit = unit.clone();
    EvaluateRequest {
        limit_id,
        claim: ClaimDto {
            value,
            unit,
            uncertainty: None,
        },
        constraints: BTreeMap::new(),
        parameters: claim
            .parameters
            .iter()
            .filter_map(|(k, p)| {
                p.value.map(|v| {
                    (
                        k.clone(),
                        ParameterDto {
                            value: v,
                            unit: p.unit.clone(),
                        },
                    )
                })
            })
            .collect(),
    }
}

/// A claim that the law question mapped to a built-in law: evaluated
/// against that law, never sent to authoring.
async fn built_in_path(
    extractor: &limina_extract::Extractor,
    text: &str,
    claim: &mut AnalyzedClaimDto,
    id: &str,
    route: &Route,
) {
    claim.trace.push(step(
        "law",
        DONE,
        format!(
            "built-in limit {id} identified by law naming; no authoring ({})",
            route.describe()
        ),
    ));
    claim.law = Some(LawDto {
        source: "built_in",
        limit_id: Some(id.to_string()),
        sentence: None,
        attempts: Vec::new(),
        comparison: None,
    });
    claim.limit_id = Some(id.to_string());
    let wanted: Vec<WantedParameter> = extractor
        .catalog()
        .limit(id)
        .map(|l| {
            l.parameters
                .iter()
                .map(|p| WantedParameter {
                    name: p.name.clone(),
                    units: p.units.clone(),
                    meaning: String::new(),
                })
                .collect()
        })
        .unwrap_or_default();
    read_parameters(extractor, text, claim, &wanted).await;
    let request = engine_request(claim, id.to_string());
    match evaluate_request(&request) {
        Ok(result) => {
            claim.status = crate::analyze::EVALUATED;
            claim.reason = None;
            claim.code = None;
            claim.result = Some(result);
        }
        Err(e) => {
            claim.result = None;
            crate::analyze::refuse(claim, &e);
        }
    }
    claim.trace.push(verdict_step(claim));
}

/// `GET /v1/pipeline`.
pub async fn info(State(state): State<AppState>) -> Json<PipelineInfoDto> {
    Json(state.pipeline_info())
}

/// `POST /v1/transcribe`: the text of an image, read by the vision model.
/// The text is then analysed like any pasted text.
pub async fn transcribe(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<TranscribeResponse>, ApiError> {
    let request: TranscribeRequest =
        serde_json::from_slice(&body).map_err(|e| ApiError::InvalidJson(e.to_string()))?;
    let vision = state.vision().ok_or(ApiError::ExtractionUnavailable)?;
    let (text, route) = transcribe_image_routed(
        vision,
        ImageInput {
            mime: request.mime,
            base64: request.base64,
        },
    )
    .await?;
    Ok(Json(TranscribeResponse {
        text,
        model: route.model,
    }))
}

/// The DSL definition behind a limit id: the embedded built-in one or the
/// authored one.
fn definition_of(state: &AppState, id: &str) -> Option<DeclarativeLimit> {
    Registry::builtin()
        .ok()
        .and_then(|r| {
            r.get(id)
                .cloned()
                .or_else(|| r.equality(id).map(|e| e.upper().clone()))
        })
        .or_else(|| state.authored(id))
}

pub(crate) fn default_range(
    state: &AppState,
    limit: &dyn Limit,
    id: &str,
    free: &str,
) -> Option<(f64, f64)> {
    let spec = limit.parameters().iter().find(|p| p.id.as_str() == free)?;
    if let Some(DomainRange::Interval(i)) = spec.plausible.as_ref().map(|d| &d.range) {
        return Some((i.lo(), i.hi()));
    }
    definition_of(state, id)?
        .definition()
        .parameters
        .iter()
        .find(|p| p.name == free)
        .and_then(|p| p.test_range)
        .map(|[lo, hi]| (lo, hi))
}

/// `POST /v1/curve`: the bound against one parameter, computed by the
/// limit itself, for the explorer. No verdict.
pub async fn curve(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<CurveResponse>, ApiError> {
    let request: CurveRequest =
        serde_json::from_slice(&body).map_err(|e| ApiError::InvalidJson(e.to_string()))?;
    Ok(Json(curve_with(&state, &request)?))
}

/// The curve itself, shared by the endpoint and its tests.
pub fn curve_with(state: &AppState, request: &CurveRequest) -> Result<CurveResponse, ApiError> {
    let id = request.limit_id.as_str();
    let equality = registry::find_equality(id);
    let limit: Box<dyn Limit> = match registry::find_limit(id) {
        Some(l) => l,
        None => Box::new(
            state
                .authored(id)
                .ok_or_else(|| ApiError::UnknownLimit(id.to_string()))?,
        ),
    };
    let constraints = graphs::constraints_of(&request.constraints);
    let free = request.free_parameter.as_str();
    if !limit.parameters().iter().any(|s| s.id.as_str() == free) {
        return Err(ApiError::UnknownFreeParameter(free.to_string()));
    }
    let fixed = graphs::fixed_si(limit.as_ref(), &request.parameters, &[free])?;
    let range = match request.range {
        Some([lo, hi]) => (lo, hi),
        None => default_range(state, limit.as_ref(), id, free)
            .ok_or_else(|| ApiError::NoSearchRange(free.to_string()))?,
    };
    let points = request.points.unwrap_or(DEFAULT_CURVE_POINTS);
    let log = match request.spacing.as_deref() {
        None | Some("linear") => false,
        Some("log") => range.0 > 0.0,
        Some("auto") => limina_dsl::wide_range(range),
        Some(other) => {
            return Err(ApiError::InvalidCurveRequest(format!(
                "spacing {other}: use linear, log or auto"
            )))
        }
    };
    if !(range.0.is_finite() && range.1.is_finite() && range.0 < range.1) {
        return Err(ApiError::InvalidCurveRequest(format!(
            "range [{}, {}]",
            range.0, range.1
        )));
    }
    if !(2..=limina_dsl::MAX_CURVE_POINTS).contains(&points) {
        return Err(ApiError::InvalidCurveRequest(format!("{points} points")));
    }
    let xs = limina_dsl::sample_points(range, points, log);
    let curve = |l: &dyn Limit, f: &BTreeMap<String, (f64, f64)>| {
        bound_curve_at(l, &constraints, f, free, &xs).map_err(graphs::curve_error)
    };
    let exact: BTreeMap<String, (f64, f64)> =
        fixed.iter().map(|(k, v)| (k.clone(), (*v, *v))).collect();
    let nominal = curve(limit.as_ref(), &exact)?;
    let direction = limit.direction();
    let lower = match &equality {
        Some(law) => Some(graphs::dto_points(
            Direction::Min,
            &curve(law.lower(), &exact)?,
        )),
        None => None,
    };
    let band = if request.uncertainties.is_empty() {
        None
    } else {
        let enclosed = graphs::enclosed_si(limit.as_ref(), request, &fixed)?;
        Some(graphs::dto_points(
            direction,
            &curve(limit.as_ref(), &enclosed)?,
        ))
    };
    let zones = match &equality {
        Some(law) => graphs::equality_zones(law, &fixed, free, &nominal),
        None => limina_dsl::validity_zones(&nominal),
    };
    Ok(CurveResponse {
        limit_id: id.to_string(),
        direction: match direction {
            Direction::Max => "Max",
            Direction::Min => "Min",
        },
        free_parameter: request.free_parameter.clone(),
        range: [range.0, range.1],
        points: graphs::dto_points(direction, &nominal),
        kind: if equality.is_some() {
            "equality"
        } else {
            "bound"
        },
        lower,
        band,
        validity_zones: zones.into_iter().map(|(a, b)| [a, b]).collect(),
        spacing: if log { "log" } else { "linear" },
    })
}
/// Name of the engine verdict, re-exported for the trace of other callers.
pub fn verdict_word(v: &limina_core::Verdict) -> &'static str {
    verdict_name(v)
}
