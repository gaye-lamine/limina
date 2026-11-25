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
    /// Parameter name to its reading. A list of readings that each carry
    /// their `name` is accepted too, as some models write it that way; a
    /// name given twice is a schema mismatch.
    #[serde(deserialize_with = "parameters_map_or_list")]
    pub parameters: BTreeMap<String, ModelParameter>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NamedParameter {
    name: String,
    value: f64,
    unit: String,
    evidence: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ParametersShape {
    Map(BTreeMap<String, ModelParameter>),
    List(Vec<NamedParameter>),
}

fn parameters_map_or_list<'de, D>(d: D) -> Result<BTreeMap<String, ModelParameter>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match ParametersShape::deserialize(d)? {
        ParametersShape::Map(m) => Ok(m),
        ParametersShape::List(list) => {
            let mut out = BTreeMap::new();
            for p in list {
                let name = p.name.clone();
                let reading = ModelParameter {
                    value: p.value,
                    unit: p.unit,
                    evidence: p.evidence,
                };
                if out.insert(name, reading).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "parameter {} given twice",
                        p.name
                    )));
                }
            }
            Ok(out)
        }
    }
}

/// The whole model answer, in the strict schema: no field more, no field less.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelOutput {
    /// The claims found.
    pub claims: Vec<ModelClaim>,
    /// Values a limit needs that the text does not give. Absent means none.
    #[serde(default)]
    pub missing: Vec<String>,
}

/// Repairs the bracket structure of a model answer, from its first `{`:
/// a closer that does not match the innermost open bracket first closes the
/// brackets left open inside it, and the brackets still open at the end are
/// closed. Text after the root object is dropped. Nothing inside a string is
/// touched and no character other than `}` and `]` is ever added, so no
/// value can change; the strict schema still applies after the repair.
/// None when there is no `{` or a string is left open.
pub fn repair_brackets(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let mut out = String::with_capacity(content.len() + 8);
    let mut stack: Vec<char> = Vec::new();
    let (mut in_string, mut escaped) = (false, false);
    for c in content[start..].chars() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '{' | '[' => {
                stack.push(c);
                out.push(c);
            }
            '}' | ']' => {
                let opener = if c == '}' { '{' } else { '[' };
                if !stack.contains(&opener) {
                    // A stray closer: dropped.
                    continue;
                }
                while let Some(top) = stack.pop() {
                    if top == opener {
                        break;
                    }
                    out.push(if top == '{' { '}' } else { ']' });
                }
                out.push(c);
            }
            _ => out.push(c),
        }
        if stack.is_empty() {
            return Some(out);
        }
    }
    if in_string {
        return None;
    }
    while let Some(top) = stack.pop() {
        out.push(if top == '{' { '}' } else { ']' });
    }
    Some(out)
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
    if let Some(object) = first_json_object(&unfenced) {
        if let Ok(out) = serde_json::from_str(object) {
            return Ok(out);
        }
    }
    // Models often drop or misplace a closing bracket: repair the brackets
    // only, then apply the same strict schema.
    let repaired = repair_brackets(&unfenced).ok_or(ExtractError::ExtractionFailed(
        "the answer holds no JSON object",
    ))?;
    let out: ModelOutput = serde_json::from_str(&repaired)
        .map_err(|_| ExtractError::ExtractionFailed("the answer does not match the schema"))?;
    // A repaired answer without any claim is most likely cut short: it is
    // refused, so the model is asked again instead of reporting "no claim".
    if out.claims.is_empty() {
        return Err(ExtractError::ExtractionFailed(
            "the answer does not match the schema",
        ));
    }
    Ok(out)
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

    #[test]
    fn a_dropped_or_misplaced_closing_bracket_is_repaired_without_changing_a_value() {
        // Seen live: the claim object is not closed before "]".
        let live = r#"{"claims":[{"limit_id":"heisenberg","quantity":"position uncertainty","value":1e-12,"unit":"m","evidence":"within 1e-12 m","constraints":{},"parameters":{"dp":{"value":1e-24,"unit":"kg*m/s","evidence":"within 1e-24 kg*m/s"}}]},"missing":[]}"#;
        let out = parse_model_output(live).expect("repaired");
        assert_eq!(out.claims.len(), 1);
        assert_eq!(out.claims[0].value, 1e-12);
        assert!(out.claims[0].parameters.contains_key("dp"));
        // Seen live: cut after the claims, no "missing".
        let cut = r#"{"claims":[{"limit_id":"heisenberg","quantity":"x","value":1e-12,"unit":"m","evidence":"e","constraints":{},"parameters":{}}]"#;
        assert_eq!(
            parse_model_output(cut).expect("closed").missing,
            Vec::<String>::new()
        );
        // Brackets inside strings are never touched; an open string is refused.
        assert_eq!(repair_brackets(r#"{"a":"x]}{"#), None);
        assert_eq!(
            repair_brackets(r#"{"a":"x]}"}"#).as_deref(),
            Some(r#"{"a":"x]}"}"#)
        );
        assert_eq!(
            repair_brackets(r#"text {"a":[1,2} tail"#).as_deref(),
            Some(r#"{"a":[1,2]}"#)
        );
        assert_eq!(repair_brackets("no json"), None);
        // The schema still applies after a repair.
        assert!(parse_model_output(r#"{"claims":[{"value":1}"#).is_err());
    }

    #[test]
    fn parameters_written_as_a_list_of_named_readings_are_accepted() {
        let list = r#"{"claims":[{"limit_id":"heisenberg","quantity":"position uncertainty","value":1e-10,"unit":"m","evidence":"1 × 10⁻¹⁰ meters","constraints":{},"parameters":[{"name":"dp","value":1e-24,"unit":"kg*m/s","evidence":"1 × 10⁻²⁴ kg·m/s"}]}],"missing":[]}"#;
        let out = parse_model_output(list).expect("list form");
        let dp = &out.claims[0].parameters["dp"];
        assert!((dp.value / 1e-24 - 1.0).abs() < 1e-12, "{}", dp.value);
        assert_eq!(dp.evidence, "1 × 10⁻²⁴ kg·m/s");
        let empty = list.replace(
            r#"[{"name":"dp","value":1e-24,"unit":"kg*m/s","evidence":"1 × 10⁻²⁴ kg·m/s"}]"#,
            "[]",
        );
        assert!(parse_model_output(&empty).expect("empty list").claims[0]
            .parameters
            .is_empty());
        let twice = list.replace(
            r#"[{"name":"dp""#,
            r#"[{"name":"dp","value":1,"unit":"x","evidence":"1"},{"name":"dp""#,
        );
        assert!(
            parse_model_output(&twice).is_err(),
            "a name given twice is refused"
        );
        let extra = list.replace(r#""name":"dp","#, r#""name":"dp","note":"x","#);
        assert!(
            parse_model_output(&extra).is_err(),
            "unknown fields are still refused"
        );
    }

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
            // "missing" may be absent (it means none); "claims" may not.
            r#"{"missing":[]}"#,
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
