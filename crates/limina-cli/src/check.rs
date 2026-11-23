//! The check of one file: each paragraph with a digit goes through the same
//! analysis as `POST /v1/analyze`, and every claim gets its position.

use limina_api::analyze::analyze_text;
use limina_api::dto::{AnalyzeResponse, AnalyzedClaimDto, EvidenceDto, ExtractionInfoDto};
use limina_api::error::ApiError;
use limina_api::AppState;
use serde::Serialize;

use crate::position::{line_col, paragraphs};
use crate::settle::{inverse, to_settle};

/// One claim with its place in the file. The claim fields are those of the
/// analysis response, with evidence offsets relative to the whole file.
#[derive(Debug, Clone, Serialize)]
pub struct LocatedClaim {
    /// The file, as given on the command line.
    pub file: String,
    /// 1-based line of the claim's evidence (of its paragraph when the
    /// claim has no traceable evidence).
    pub line: usize,
    /// 1-based column, in characters.
    pub column: usize,
    /// The analysed claim.
    #[serde(flatten)]
    pub claim: AnalyzedClaimDto,
    /// What the author would have to state to settle it.
    pub to_settle: Option<String>,
    /// The inverse solver's sentence, when it has one.
    pub inverse: Option<String>,
}

/// A paragraph that could not be analysed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParagraphError {
    /// The file, as given on the command line.
    pub file: String,
    /// 1-based line where the paragraph starts.
    pub line: usize,
    /// 1-based column of its first character.
    pub column: usize,
    /// Stable error code of the API.
    pub code: &'static str,
    /// Message for a human; never the key, never the model output.
    pub message: String,
}

/// The whole check: the fields of the analysis response, plus the file and
/// the paragraphs that could not be analysed.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The file, as given on the command line.
    pub file: String,
    /// Every claim, in file order.
    pub claims: Vec<LocatedClaim>,
    /// Values the model said the text does not give.
    pub missing: Vec<String>,
    /// How the extraction ran, over all paragraphs.
    pub extraction: ExtractionInfoDto,
    /// Paragraphs that could not be analysed; any entry means exit code 2.
    pub errors: Vec<ParagraphError>,
}

impl Report {
    /// Number of claims the engine found IMPOSSIBLE.
    pub fn impossible(&self) -> usize {
        self.count("IMPOSSIBLE")
    }

    /// Number of claims with this engine verdict.
    pub fn count(&self, verdict: &str) -> usize {
        self.claims
            .iter()
            .filter(|c| c.claim.result.as_ref().map(|r| r.verdict) == Some(verdict))
            .count()
    }

    /// Number of claims that need input before the engine can decide.
    pub fn needs_input(&self) -> usize {
        self.claims
            .iter()
            .filter(|c| c.claim.status == "needs_input")
            .count()
    }
}

fn shift(evidence: &mut Option<EvidenceDto>, by: usize) {
    if let Some(e) = evidence {
        e.start += by;
        e.end += by;
    }
}

/// Model answers tried per paragraph when the answer is unusable
/// (`ExtractionFailed`: no JSON, cut at the token limit, not the schema).
/// Any other error is returned at once. A retry asks the model again for
/// the same text; it never touches a value or a verdict.
pub const ATTEMPTS_PER_PARAGRAPH: usize = 3;

async fn analyze_paragraph(state: &AppState, text: &str) -> Result<AnalyzeResponse, ApiError> {
    let mut attempt = 1;
    loop {
        match analyze_text(state, text).await {
            Err(ApiError::ExtractionFailed(_)) if attempt < ATTEMPTS_PER_PARAGRAPH => attempt += 1,
            other => return other,
        }
    }
}

/// Whether the next paragraphs can still be analysed after this error: an
/// unusable answer or a too long paragraph concerns one paragraph only; no
/// key, a provider failure or the rate limit would fail every next call.
fn paragraph_only(error: &ApiError) -> bool {
    matches!(
        error,
        ApiError::ExtractionFailed(_) | ApiError::TextTooLong { .. } | ApiError::EmptyText
    )
}

/// Checks `text`, read from `file`. `model` names the model when no
/// paragraph needed a call. A paragraph that cannot be analysed is listed
/// in `errors`; the check goes on unless the error would repeat.
pub async fn check(state: &AppState, file: &str, text: &str, model: &str) -> Report {
    let mut report = Report {
        file: file.to_string(),
        claims: Vec::new(),
        missing: Vec::new(),
        extraction: ExtractionInfoDto {
            model: model.to_string(),
            cached: true,
            ms: 0,
        },
        errors: Vec::new(),
    };
    for paragraph in paragraphs(text) {
        if !paragraph.text.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }
        let response = match analyze_paragraph(state, paragraph.text).await {
            Ok(r) => r,
            Err(error) => {
                let (line, column) = line_col(text, paragraph.start);
                let go_on = paragraph_only(&error);
                report.errors.push(ParagraphError {
                    file: file.to_string(),
                    line,
                    column,
                    code: error.code(),
                    message: error.to_string(),
                });
                if go_on {
                    continue;
                }
                break;
            }
        };
        report.extraction.model = response.extraction.model;
        report.extraction.cached &= response.extraction.cached;
        report.extraction.ms += response.extraction.ms;
        report.missing.extend(response.missing);
        for mut claim in response.claims {
            shift(&mut claim.evidence, paragraph.start);
            for p in claim.parameters.values_mut() {
                shift(&mut p.evidence, paragraph.start);
            }
            let at = claim.evidence.as_ref().map_or(paragraph.start, |e| e.start);
            let (line, column) = line_col(text, at);
            report.claims.push(LocatedClaim {
                file: file.to_string(),
                line,
                column,
                to_settle: to_settle(&claim),
                inverse: inverse(state, &claim),
                claim,
            });
        }
    }
    report
}
