//! Reading the model answer into the strict output schema.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::ExtractError;
use crate::provider::ChatReply;

/// A parameter as the model read it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelParameter {
    /// Number as written.
    pub value: f64,
    /// Unit as written.
    pub unit: String,
    /// Quotation that should contain the number.
    pub evidence: String,
}

/// A claim as the model read it. Nothing here is trusted yet.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelClaim {
    /// Limit id, or null when the model does not know.
    pub limit_id: Option<String>,
    /// What the claim is about, in words.
    pub quantity: String,
    /// Number as written.
    pub value: f64,
    /// Unit as written.
    pub unit: String,
    /// Quotation that should contain the number.
    pub evidence: String,
    /// Assumption id to option id.
    pub constraints: BTreeMap<String, String>,
    /// Parameter name to its reading.
    pub parameters: BTreeMap<String, ModelParameter>,
}

/// The whole model answer, in the strict schema: no field more, no field less.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelOutput {
    /// The claims found.
    pub claims: Vec<ModelClaim>,
    /// Values a limit needs that the text does not give.
    pub missing: Vec<String>,
}

/// Reads `message.content` only. Empty content, `finish_reason` "length",
/// non-JSON or a schema mismatch is `ExtractionFailed`.
pub fn read_reply(reply: &ChatReply) -> Result<ModelOutput, ExtractError> {
    if reply.finish_reason.as_deref() == Some("length") {
        return Err(ExtractError::ExtractionFailed(
            "the answer was cut at the token limit",
        ));
    }
    let content = reply.content.as_deref().unwrap_or("");
    if content.trim().is_empty() {
        return Err(ExtractError::ExtractionFailed("the answer was empty"));
    }
    parse_model_output(content)
}

/// Strips markdown code fences, then parses the first JSON object.
pub fn parse_model_output(content: &str) -> Result<ModelOutput, ExtractError> {
    let unfenced = strip_fences(content);
    let object = first_json_object(&unfenced).ok_or(ExtractError::ExtractionFailed(
        "the answer holds no JSON object",
    ))?;
    serde_json::from_str(object)
        .map_err(|_| ExtractError::ExtractionFailed("the answer does not match the schema"))
}

/// Removes ``` fence lines (with or without a language tag).
pub(crate) fn strip_fences(content: &str) -> String {
    content
        .lines()
        .filter(|l| !l.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first balanced `{...}`, aware of strings and escapes.
pub(crate) fn first_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in text[start..].char_indices() {
        if in_string {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&text[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{"claims":[{"limit_id":"carnot","quantity":"efficiency","value":60,"unit":"%","evidence":"60% efficiency","constraints":{},"parameters":{"Th":{"value":600,"unit":"K","evidence":"600 K"}}}],"missing":[]}"#;

    fn reply(content: Option<&str>, finish: &str) -> ChatReply {
        ChatReply {
            content: content.map(str::to_string),
            finish_reason: Some(finish.to_string()),
        }
    }

    #[test]
    fn valid_json() {
        let out = read_reply(&reply(Some(VALID), "stop")).expect("valid");
        assert_eq!(out.claims.len(), 1);
        assert_eq!(out.claims[0].parameters["Th"].value, 600.0);
    }

    #[test]
    fn json_inside_code_fences_with_prose_around() {
        let fenced = format!("Here you go:\n```json\n{VALID}\n```\nDone.");
        assert_eq!(
            read_reply(&reply(Some(&fenced), "stop")),
            read_reply(&reply(Some(VALID), "stop"))
        );
    }

    #[test]
    fn the_first_object_wins_and_braces_in_strings_are_ignored() {
        let two = r#"{"claims":[],"missing":["a } b"]} {"claims":[{"oops":1}],"missing":[]}"#;
        assert_eq!(
            parse_model_output(two).expect("first").missing,
            vec!["a } b"]
        );
    }

    #[test]
    fn empty_content_is_extraction_failed() {
        for c in [None, Some(""), Some("   ")] {
            assert!(matches!(
                read_reply(&reply(c, "stop")),
                Err(ExtractError::ExtractionFailed(_))
            ));
        }
    }

    #[test]
    fn finish_reason_length_is_extraction_failed() {
        assert_eq!(
            read_reply(&reply(Some(VALID), "length")),
            Err(ExtractError::ExtractionFailed(
                "the answer was cut at the token limit"
            ))
        );
    }

    #[test]
    fn non_json_and_schema_mismatches_are_extraction_failed() {
        for c in [
            "no json here",
            "{ not json",
            r#"{"claims":[]}"#,
            r#"{"claims":[],"missing":[],"extra":1}"#,
            r#"{"claims":[{"limit_id":"carnot","quantity":"q","value":"60","unit":"%","evidence":"e","constraints":{},"parameters":{}}],"missing":[]}"#,
        ] {
            assert!(
                matches!(
                    parse_model_output(c),
                    Err(ExtractError::ExtractionFailed(_))
                ),
                "{c}"
            );
        }
    }
}
