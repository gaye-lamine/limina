//! The five tools. Each one builds a request for the Limina API, sends it, and
//! returns the API's answer. No verdict, bound or threshold is computed here.

use std::collections::BTreeMap;
use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::client::{ApiClient, ApiReply, ClientError};

/// The message `analyze_text` gives when the API has no extraction key.
pub const EXTRACTION_UNAVAILABLE: &str = "Extraction is unavailable on this Limina server: no language-model key is configured there, so a text cannot be read into claims. \
The deterministic tools still work: read the figures yourself and call evaluate_claim or solve_threshold.";

/// Fields of a limit kept by `list_limits`.
const LIST_FIELDS: [&str; 7] = [
    "id",
    "title",
    "direction",
    "claim_unit",
    "formula",
    "statement",
    "classification",
];

/// The MCP server: a handle on the Limina API.
#[derive(Debug, Clone)]
pub struct LiminaMcp {
    api: Arc<ApiClient>,
}

/// Input of `explain_limit`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExplainParams {
    /// The id of a limit, as listed by `list_limits`, for example `carnot`.
    pub id: String,
}

/// A parameter value with its unit.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ParameterInput {
    /// The value, in `unit`.
    pub value: f64,
    /// A unit symbol, for example `K`, `m`, `W` or `dimensionless`.
    pub unit: String,
}

/// A claimed value.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ClaimInput {
    /// The claimed value, in `unit`.
    pub value: f64,
    /// A unit symbol, for example `dimensionless`, `W` or `m/s`.
    pub unit: String,
    /// The uncertainty of the claim, in the same unit; absent means none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<f64>,
}

/// Input of `evaluate_claim`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EvaluateParams {
    /// The id of a limit, for example `carnot`.
    pub limit_id: String,
    /// The claimed value, in `unit`.
    pub value: f64,
    /// A unit symbol, for example `dimensionless`, `W` or `m/s`.
    pub unit: String,
    /// The uncertainty of the claim, in the same unit; absent means none.
    #[serde(default)]
    pub uncertainty: Option<f64>,
    /// Assumption id to the option ids the text allows, for example
    /// `{"conversion_process": ["heat_engine_cycle"]}`. An assumption the text
    /// does not state is left out.
    #[serde(default)]
    pub constraints: BTreeMap<String, Vec<String>>,
    /// Parameter name to its value and unit, for example
    /// `{"Th": {"value": 600, "unit": "K"}}`. Temperatures are in kelvin.
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterInput>,
}

/// Input of `solve_threshold`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SolveParams {
    /// The id of a limit, for example `carnot`.
    pub limit_id: String,
    /// The claim to match.
    pub claim: ClaimInput,
    /// The parameter to solve for, for example `Th`.
    pub free_parameter: String,
    /// Assumption id to the option ids the text allows.
    #[serde(default)]
    pub constraints: BTreeMap<String, Vec<String>>,
    /// Every parameter other than the free one, with its value and unit.
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterInput>,
}

/// Input of `analyze_text`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnalyzeParams {
    /// The text to read the claims from, at most 5000 characters.
    pub text: String,
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn success(value: &Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(pretty(value))])
}

fn failure(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

/// The code and the message of an API error body `{"error": {"code", "message"}}`.
fn error_parts(body: &Value) -> (String, String) {
    let field = |name: &str| {
        body.pointer(&format!("/error/{name}"))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    (
        field("code").unwrap_or_else(|| "UnknownError".to_string()),
        field("message").unwrap_or_else(|| body.to_string()),
    )
}

fn api_error(reply: &ApiReply) -> CallToolResult {
    let (code, message) = error_parts(&reply.body);
    failure(format!(
        "The Limina API answered HTTP {} ({code}): {message}",
        reply.status
    ))
}

/// A successful reply as a result, anything else as an error result.
fn pass_on(call: Result<ApiReply, ClientError>) -> CallToolResult {
    match call {
        Err(e) => failure(e.to_string()),
        Ok(reply) if (200..300).contains(&reply.status) => success(&reply.body),
        Ok(reply) => api_error(&reply),
    }
}

/// The body of `POST /v1/evaluate`.
fn evaluate_body(p: &EvaluateParams) -> Value {
    json!({
        "limit_id": p.limit_id,
        "claim": ClaimInput { value: p.value, unit: p.unit.clone(), uncertainty: p.uncertainty },
        "constraints": p.constraints,
        "parameters": p.parameters,
    })
}

/// The body of `POST /v1/solve`.
fn solve_body(p: &SolveParams) -> Value {
    json!({
        "limit_id": p.limit_id,
        "claim": p.claim,
        "free_parameter": p.free_parameter,
        "constraints": p.constraints,
        "parameters": p.parameters,
    })
}

/// The limits of a `GET /v1/limits` body.
fn limits_of(body: &Value) -> Vec<&Value> {
    body.get("limits")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

#[tool_router]
impl LiminaMcp {
    /// A server that calls the Limina API behind `api`.
    pub fn new(api: ApiClient) -> LiminaMcp {
        LiminaMcp { api: Arc::new(api) }
    }

    #[tool(
        description = "List the physical limits in the Limina library: id, title, direction (Max, Min or Equal), claim unit, formula and a one-sentence statement. Call it first to find a limit id."
    )]
    async fn list_limits(&self) -> Result<CallToolResult, ErrorData> {
        let reply = match self.api.get("/v1/limits").await {
            Ok(r) if (200..300).contains(&r.status) => r,
            other => return Ok(pass_on(other)),
        };
        let listed: Vec<Value> = limits_of(&reply.body)
            .into_iter()
            .map(|limit| {
                let kept: serde_json::Map<String, Value> = LIST_FIELDS
                    .iter()
                    .filter_map(|f| limit.get(*f).map(|v| ((*f).to_string(), v.clone())))
                    .collect();
                Value::Object(kept)
            })
            .collect();
        Ok(success(&json!({ "count": listed.len(), "limits": listed })))
    }

    #[tool(
        description = "Describe one limit in full: formula, statement, assumptions with their options, parameters with units, validity domain, limitations and references. Takes an id from list_limits."
    )]
    async fn explain_limit(
        &self,
        Parameters(p): Parameters<ExplainParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let reply = match self.api.get("/v1/limits").await {
            Ok(r) if (200..300).contains(&r.status) => r,
            other => return Ok(pass_on(other)),
        };
        let limits = limits_of(&reply.body);
        if let Some(found) = limits
            .iter()
            .find(|l| l.get("id").and_then(Value::as_str) == Some(p.id.as_str()))
        {
            return Ok(success(found));
        }
        let known: Vec<&str> = limits
            .iter()
            .filter_map(|l| l.get("id").and_then(Value::as_str))
            .collect();
        Ok(failure(format!(
            "Unknown limit id \"{}\". Known ids: {}.",
            p.id,
            known.join(", ")
        )))
    }

    #[tool(
        description = "Check a claimed value against a limit with the Limina deterministic engine. Returns the engine's own answer unchanged: verdict (IMPOSSIBLE, CONDITIONAL, COMPATIBLE or INSUFFICIENT_DATA), wording, bounds, margin, configurations and missing parameters. This tool computes nothing itself. constraints: assumption id to the option ids the text allows. parameters: name to value and unit, temperatures in kelvin."
    )]
    async fn evaluate_claim(
        &self,
        Parameters(p): Parameters<EvaluateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(pass_on(
            self.api.post("/v1/evaluate", &evaluate_body(&p)).await,
        ))
    }

    #[tool(
        description = "Find the value of one free parameter at which a limit's bound equals the claim, with the engine's verdict just below and just above it. Answers questions such as: how hot must the hot reservoir be for this efficiency to be possible? The search is done by the Limina engine, not by this tool."
    )]
    async fn solve_threshold(
        &self,
        Parameters(p): Parameters<SolveParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(pass_on(self.api.post("/v1/solve", &solve_body(&p)).await))
    }

    #[tool(
        description = "Read the numeric performance claims out of a paragraph (at most 5000 characters) with the language model behind the Limina API, then check each one against the library. Needs extraction on the Limina server; when it is not available this tool says so explicitly."
    )]
    async fn analyze_text(
        &self,
        Parameters(p): Parameters<AnalyzeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let call = self
            .api
            .post("/v1/analyze", &json!({ "text": p.text }))
            .await;
        if let Ok(reply) = &call {
            let (code, _) = error_parts(&reply.body);
            if reply.status == 503 && code == "ExtractionUnavailable" {
                return Ok(failure(EXTRACTION_UNAVAILABLE));
            }
        }
        Ok(pass_on(call))
    }
}

#[tool_handler(
    name = "limina",
    version = "0.1.0",
    instructions = "Limina checks physical performance claims against fundamental limits. Every verdict comes from the deterministic engine of the Limina API; these tools pass its answer on and never compute or soften a verdict. Start with list_limits."
)]
impl ServerHandler for LiminaMcp {}
