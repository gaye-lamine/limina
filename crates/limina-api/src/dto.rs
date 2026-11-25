//! Request and response types of the API (JSON shapes).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Body of `POST /v1/evaluate`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluateRequest {
    /// Id of the limit: `betz` or `carnot`.
    pub limit_id: String,
    /// The claim to check.
    pub claim: ClaimDto,
    /// Assumption id to the allowed option ids.
    #[serde(default)]
    pub constraints: BTreeMap<String, Vec<String>>,
    /// Parameter name to its value.
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterDto>,
}

/// The claimed value, in `unit`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimDto {
    /// The claimed value.
    pub value: f64,
    /// Unit symbol, from the engine's unit table.
    pub unit: String,
    /// Uncertainty, in the same unit as `value`. Absent means zero.
    #[serde(default)]
    pub uncertainty: Option<f64>,
}

/// A parameter value with its unit.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterDto {
    /// The value.
    pub value: f64,
    /// Unit symbol, from the engine's unit table.
    pub unit: String,
}

/// Body of a successful `POST /v1/evaluate`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvaluateResponse {
    /// `IMPOSSIBLE`, `CONDITIONAL`, `COMPATIBLE` or `INSUFFICIENT_DATA`.
    pub verdict: &'static str,
    /// Id of the limit that was evaluated.
    pub limit_id: String,
    /// Version of the engine build that computed the verdict.
    pub engine_version: &'static str,
    /// One of the four fixed wording formulas.
    pub wording: String,
    /// Bounds under both enclosures; absent when the engine computed none.
    pub bounds: Option<BoundsDto>,
    /// Members of C*; non-empty only for `CONDITIONAL`.
    pub configurations: Vec<ConfigurationDto>,
    /// Missing parameters; non-empty only for `INSUFFICIENT_DATA`.
    pub missing_parameters: Vec<MissingParameterDto>,
}

/// Bounds under the permissive and the restrictive enclosure.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoundsDto {
    /// Permissive enclosure.
    pub permissive: EnclosureDto,
    /// Restrictive enclosure.
    pub restrictive: EnclosureDto,
}

/// `B_min`, `B_max` and margin under one enclosure, in SI base units.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnclosureDto {
    /// Smallest bound over the configuration set.
    pub b_min: f64,
    /// Largest bound over the configuration set.
    pub b_max: f64,
    /// Claim-to-bound ratio range; absent when the engine computed none.
    pub margin: Option<MarginDto>,
}

/// Hull of the claim-to-bound ratios.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MarginDto {
    /// Smallest ratio.
    pub low: f64,
    /// Largest ratio.
    pub high: f64,
}

/// One member of C*.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConfigurationDto {
    /// Assumption id to option id.
    pub assumptions: BTreeMap<String, String>,
    /// `Certain` or `Depends`.
    pub support: &'static str,
    /// Parameters the support depends on; empty for `Certain`.
    pub depends_on: Vec<String>,
}

/// A required parameter that was not supplied.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MissingParameterDto {
    /// Parameter id.
    pub id: String,
    /// `NoDomain` or `HasDomain`.
    pub status: &'static str,
}

/// Body of `GET /v1/limits`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LimitsResponse {
    /// The served limits.
    pub limits: Vec<LimitDto>,
}

/// Description of one limit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LimitDto {
    /// Limit id.
    pub id: String,
    /// `Max` or `Min`.
    pub direction: &'static str,
    /// Display title, for example `Betz limit`.
    pub title: String,
    /// The bound as plain text, for example `Cp <= 16/27`.
    pub formula: String,
    /// One plain sentence stating what the limit says.
    pub statement: String,
    /// Canonical unit symbol of the bounded quantity.
    pub claim_unit: String,
    /// Declared assumptions.
    pub assumptions: Vec<AssumptionDto>,
    /// Declared parameters.
    pub parameters: Vec<ParameterSpecDto>,
    /// Display label of each assumption id of this limit.
    pub assumption_labels: BTreeMap<String, String>,
    /// Display label of each option id of this limit.
    pub option_labels: BTreeMap<String, String>,
    /// Display label of each parameter id of this limit.
    pub parameter_labels: BTreeMap<String, String>,
    /// "draft" for a limit authored from a text; absent for the built-in limits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// `draft_agreed` for a law written by two models that agree; absent
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement: Option<String>,
    /// Kind of law (`theorem`, `theoretical_bound`, ...); absent when the
    /// definition does not say.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    /// References of the law; absent when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    /// Where the law applies, in plain text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validity_domain: Option<String>,
    /// Limits of application, in plain text.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
}

/// An assumption and its options.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssumptionDto {
    /// Assumption id.
    pub id: String,
    /// Option ids of its domain.
    pub options: Vec<String>,
}

/// A declared parameter.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParameterSpecDto {
    /// Parameter id.
    pub id: String,
    /// `AbsoluteTemperature`, `TemperatureDifference` or `Plain`.
    pub kind: &'static str,
    /// Whether the bound needs it.
    pub required: bool,
    /// Allowed unit symbols for this parameter.
    pub units: Vec<String>,
}

/// Body of `POST /v1/analyze`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeRequest {
    /// The text to read the claims from, at most 5000 characters.
    pub text: String,
}

/// Where an evidence sits in the text, in characters: `start` inclusive,
/// `end` exclusive.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvidenceDto {
    /// The evidence, exactly as in the text.
    pub text: String,
    /// Character offset of its first character.
    pub start: usize,
    /// Character offset just after its last character.
    pub end: usize,
}

/// A parameter read from the text, after the deterministic checks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalyzedParameterDto {
    /// The value sent to the engine; null when not traceable or not allowed.
    pub value: Option<f64>,
    /// The number the model read.
    pub raw_value: f64,
    /// The unit as read.
    pub unit: String,
    /// Where the evidence is; null when not traceable.
    pub evidence: Option<EvidenceDto>,
    /// What was found wrong.
    pub flags: Vec<String>,
}

/// One claim read from the text, checked, and evaluated when possible.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalyzedClaimDto {
    /// Limit id as read; null when the model did not know.
    pub limit_id: Option<String>,
    /// What the claim is about, in words.
    pub quantity: String,
    /// The value sent to the engine, after normalization; null when not traceable.
    pub value: Option<f64>,
    /// Unit after normalization.
    pub unit: String,
    /// The number the model read.
    pub raw_value: f64,
    /// The unit as read.
    pub raw_unit: String,
    /// Where the evidence is; null when not traceable.
    pub evidence: Option<EvidenceDto>,
    /// Constraints the text states (assumption id to option id).
    pub constraints: BTreeMap<String, String>,
    /// Parameters, after the checks.
    pub parameters: BTreeMap<String, AnalyzedParameterDto>,
    /// Normalization notes, for example "60 % becomes 0.60".
    pub notes: Vec<String>,
    /// What was found wrong.
    pub flags: Vec<String>,
    /// `evaluated` or `needs_input`.
    pub status: &'static str,
    /// The engine result, when evaluated; the same shape as `POST /v1/evaluate`.
    pub result: Option<EvaluateResponse>,
    /// Why the claim needs input, when it does.
    pub reason: Option<String>,
    /// Stable error code of an engine refusal, when that is the reason.
    pub code: Option<&'static str>,
    /// What the pipeline did with this claim, step by step. Failures are
    /// listed, never hidden.
    pub trace: Vec<TraceStepDto>,
    /// The law the claim was checked against, and where it comes from.
    pub law: Option<LawDto>,
}

/// One step of the pipeline for one claim.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceStepDto {
    /// Step name, for example `extraction`, `law`, `comparison`.
    pub step: String,
    /// `done`, `failed` or `skipped`.
    pub status: &'static str,
    /// Short factual detail.
    pub detail: String,
}

/// The numerical comparison of two model-written definitions.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparisonDto {
    /// Samples evaluated.
    pub samples: usize,
    /// Largest relative difference seen.
    pub max_relative_difference: f64,
    /// Why they disagree; null when they agree.
    pub reason: Option<String>,
}

/// One model's attempt at writing the law.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LawAttemptDto {
    /// Model id.
    pub model: String,
    /// Whether it wrote a definition that passed the DSL checks.
    pub ok: bool,
    /// Answers used.
    pub attempts: usize,
    /// Every error sent back to the model, or the final error.
    pub errors: Vec<String>,
    /// The definition, when it passed.
    pub definition: Option<limina_dsl::LimitDefinition>,
}

/// The law of a claim.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LawDto {
    /// `built_in`, `draft_agreed`, `draft_disputed`, `not_authored` or
    /// `no_law_identified`.
    pub source: &'static str,
    /// The limit evaluated, when there is one.
    pub limit_id: Option<String>,
    /// The law as one sentence, when a model named it.
    pub sentence: Option<String>,
    /// The two authoring attempts, for an AI-written law.
    pub attempts: Vec<LawAttemptDto>,
    /// The comparison, when both definitions passed.
    pub comparison: Option<ComparisonDto>,
}

/// Body of `POST /v1/transcribe`: an image as base64.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscribeRequest {
    /// `image/png`, `image/jpeg` or `image/webp`.
    pub mime: String,
    /// The image bytes, base64 encoded (no `data:` prefix).
    pub base64: String,
}

/// Body of a successful `POST /v1/transcribe`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TranscribeResponse {
    /// The text read from the image.
    pub text: String,
    /// The vision model used.
    pub model: String,
}

/// Body of `POST /v1/curve`. Parameter values in the given units; `range`
/// in SI units (kelvin for temperatures).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurveRequest {
    /// A built-in or authored limit id.
    pub limit_id: String,
    /// Assumption id to the allowed option ids.
    #[serde(default)]
    pub constraints: BTreeMap<String, Vec<String>>,
    /// Every parameter other than the free one, fixed.
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterDto>,
    /// The parameter on the horizontal axis.
    pub free_parameter: String,
    /// Range of the free parameter; absent uses the plausible domain, else
    /// the test range of the definition.
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    /// Number of points, 2 to 1000; default 101.
    #[serde(default)]
    pub points: Option<usize>,
}

/// One point of a curve: the bound range over the configurations, null
/// where the bound is undefined.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CurvePointDto {
    /// Value of the free parameter, SI.
    pub x: f64,
    /// Smallest bound over the configurations.
    pub b_min: Option<f64>,
    /// Largest bound over the configurations.
    pub b_max: Option<f64>,
    /// The bound that decides IMPOSSIBLE: `b_max` for a Max limit, `b_min`
    /// for a Min limit.
    pub decisive: Option<f64>,
}

/// Body of a successful `POST /v1/curve`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CurveResponse {
    /// The limit.
    pub limit_id: String,
    /// `Max` or `Min`.
    pub direction: &'static str,
    /// The free parameter.
    pub free_parameter: String,
    /// Range used, SI.
    pub range: [f64; 2],
    /// The points, in order of `x`.
    pub points: Vec<CurvePointDto>,
}

/// Body of `GET /v1/pipeline`: what the analysis pipeline can do here.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PipelineInfoDto {
    /// Claims are read from text.
    pub extraction: bool,
    /// Claims without a built-in limit get a law written by two models.
    pub law_authoring: bool,
    /// Images can be transcribed.
    pub vision: bool,
    /// Model ids: extraction and model A, model B, vision.
    pub models: Vec<String>,
}

/// How the extraction ran.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExtractionInfoDto {
    /// Model id.
    pub model: String,
    /// Whether the model answer came from the cache.
    pub cached: bool,
    /// Time spent, in milliseconds.
    pub ms: u64,
}

/// Body of a successful `POST /v1/analyze`. Never the key, never the raw
/// model output.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalyzeResponse {
    /// The claims read from the text.
    pub claims: Vec<AnalyzedClaimDto>,
    /// Values the model said the text does not give.
    pub missing: Vec<String>,
    /// How the extraction ran.
    pub extraction: ExtractionInfoDto,
}
