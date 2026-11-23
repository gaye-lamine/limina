//! Deterministic checks on the model answer. No language model here.
//!
//! - Every evidence must be an exact substring of the text and every number
//!   must appear in its evidence; otherwise the value is dropped and flagged.
//! - Units are normalized ("%" to a fraction) or flagged, never guessed.
//! - A constraint is kept only if the text states its option.
//! - Temperatures are passed as read; the engine decides their order.

use std::collections::BTreeMap;

use crate::catalog::{Catalog, CatalogLimit};
use crate::parse::{ModelClaim, ModelOutput, ModelParameter};

/// Flag: an evidence is not in the text, or its number is not in the evidence.
pub const NOT_TRACEABLE: &str = "not traceable to the text";
/// Flag: a unit the limit does not allow; it is never guessed.
pub const UNIT_NOT_UNDERSTOOD: &str = "unit not understood";
/// Flag: a constraint the text does not state; it is dropped.
pub const CONSTRAINT_NOT_STATED: &str = "constraint not stated in the text";
/// Flag: the limit id is null or not served.
pub const LIMIT_NOT_RECOGNIZED: &str = "limit not recognized";
/// Flag: a parameter the limit does not declare; it is dropped.
pub const PARAMETER_NOT_DECLARED: &str = "parameter not declared for this limit";
/// Reason a claim with no traceable value is not evaluated.
pub const VALUE_NOT_TRACEABLE: &str = "claim value not traceable to the text";

/// Where an evidence sits in the text, in characters: `start` inclusive,
/// `end` exclusive.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    /// The evidence, exactly as in the text.
    pub text: String,
    /// Character offset of its first character.
    pub start: usize,
    /// Character offset just after its last character.
    pub end: usize,
}

/// A parameter after the checks.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckedParameter {
    /// The value, only when traceable to the text and declared for the limit.
    pub value: Option<f64>,
    /// The number the model read.
    pub raw_value: f64,
    /// The unit as read.
    pub unit: String,
    /// Where the evidence is, when it is in the text.
    pub evidence: Option<Span>,
    /// What was found wrong.
    pub flags: Vec<String>,
}

/// A claim after the checks.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckedClaim {
    /// Limit id as read.
    pub limit_id: Option<String>,
    /// What the claim is about, in words.
    pub quantity: String,
    /// Normalized value, only when traceable and its unit understood.
    pub value: Option<f64>,
    /// The number the model read.
    pub raw_value: f64,
    /// Unit after normalization (the limit's claim unit), or as read when not understood.
    pub unit: String,
    /// The unit as read.
    pub raw_unit: String,
    /// Where the evidence is, when it is in the text.
    pub evidence: Option<Span>,
    /// Constraints the text states.
    pub constraints: BTreeMap<String, String>,
    /// Parameters after the checks.
    pub parameters: BTreeMap<String, CheckedParameter>,
    /// Normalization notes, for example "60 % becomes 0.60".
    pub notes: Vec<String>,
    /// What was found wrong.
    pub flags: Vec<String>,
}

impl CheckedClaim {
    /// Why the claim cannot go to the engine, or None when it can.
    pub fn not_evaluable_reason(&self) -> Option<&'static str> {
        if self.flags.iter().any(|f| f == LIMIT_NOT_RECOGNIZED) {
            Some(LIMIT_NOT_RECOGNIZED)
        } else if self.flags.iter().any(|f| f == UNIT_NOT_UNDERSTOOD) {
            Some(UNIT_NOT_UNDERSTOOD)
        } else if self.value.is_none() {
            Some(VALUE_NOT_TRACEABLE)
        } else {
            None
        }
    }

    /// Whether the claim can go to the engine.
    pub fn evaluable(&self) -> bool {
        self.not_evaluable_reason().is_none()
    }

    /// The parameters that may go to the engine: traceable and declared.
    pub fn traced_parameters(&self) -> BTreeMap<String, f64> {
        self.parameters
            .iter()
            .filter_map(|(k, p)| p.value.map(|v| (k.clone(), v)))
            .collect()
    }
}

/// Runs every check on the model answer, against the text it came from.
pub fn check(output: &ModelOutput, text: &str, catalog: &Catalog) -> Vec<CheckedClaim> {
    output
        .claims
        .iter()
        .map(|c| check_claim(c, text, catalog))
        .collect()
}

fn push_once(flags: &mut Vec<String>, flag: &str) {
    if !flags.iter().any(|f| f == flag) {
        flags.push(flag.to_string());
    }
}

fn check_claim(claim: &ModelClaim, text: &str, catalog: &Catalog) -> CheckedClaim {
    let mut flags = Vec::new();
    let mut notes = Vec::new();
    let limit = claim.limit_id.as_deref().and_then(|id| catalog.limit(id));
    if limit.is_none() {
        push_once(&mut flags, LIMIT_NOT_RECOGNIZED);
    }
    let evidence = locate(text, &claim.evidence);
    let traced = evidence.is_some() && number_in(&claim.evidence, claim.value);
    if !traced {
        push_once(&mut flags, NOT_TRACEABLE);
    }
    let (value, unit) = match limit {
        Some(l) => match normalize_claim_unit(l, &claim.unit, claim.value) {
            Some((v, u, note)) => {
                if let (true, Some(n)) = (traced, note) {
                    notes.push(n);
                }
                (traced.then_some(v), u)
            }
            None => {
                push_once(&mut flags, UNIT_NOT_UNDERSTOOD);
                (None, claim.unit.clone())
            }
        },
        None => (None, claim.unit.clone()),
    };
    let parameters = claim
        .parameters
        .iter()
        .map(|(name, p)| {
            let checked = check_parameter(name, p, text, limit);
            if checked.flags.iter().any(|f| f == UNIT_NOT_UNDERSTOOD) {
                push_once(&mut flags, UNIT_NOT_UNDERSTOOD);
            }
            (name.clone(), checked)
        })
        .collect();
    let mut constraints = BTreeMap::new();
    for (assumption, option) in &claim.constraints {
        if limit.is_some_and(|l| stated(text, l, assumption, option)) {
            constraints.insert(assumption.clone(), option.clone());
        } else {
            push_once(&mut flags, CONSTRAINT_NOT_STATED);
        }
    }
    CheckedClaim {
        limit_id: claim.limit_id.clone(),
        quantity: claim.quantity.clone(),
        value,
        raw_value: claim.value,
        unit,
        raw_unit: claim.unit.clone(),
        evidence: if traced { evidence } else { None },
        constraints,
        parameters,
        notes,
        flags,
    }
}

fn check_parameter(
    name: &str,
    p: &ModelParameter,
    text: &str,
    limit: Option<&CatalogLimit>,
) -> CheckedParameter {
    let mut flags = Vec::new();
    let evidence = locate(text, &p.evidence);
    let traced = evidence.is_some() && number_in(&p.evidence, p.value);
    if !traced {
        push_once(&mut flags, NOT_TRACEABLE);
    }
    let declared = limit.and_then(|l| l.parameter(name));
    match declared {
        None if limit.is_some() => push_once(&mut flags, PARAMETER_NOT_DECLARED),
        Some(spec) if !spec.units.iter().any(|u| u == &p.unit) => {
            push_once(&mut flags, UNIT_NOT_UNDERSTOOD)
        }
        _ => {}
    }
    let ok = traced && declared.is_some_and(|spec| spec.units.iter().any(|u| u == &p.unit));
    CheckedParameter {
        value: ok.then_some(p.value),
        raw_value: p.value,
        unit: p.unit.clone(),
        evidence: if traced { evidence } else { None },
        flags,
    }
}

/// The value in the limit's claim unit, the unit, and a note when converted.
/// None when the unit is not one the limit allows.
fn normalize_claim_unit(
    limit: &CatalogLimit,
    unit: &str,
    value: f64,
) -> Option<(f64, String, Option<String>)> {
    let unit = unit.trim();
    let dimensionless = limit.claim_unit == "dimensionless";
    if unit == limit.claim_unit || (dimensionless && unit.is_empty()) {
        return Some((value, limit.claim_unit.clone(), None));
    }
    if dimensionless && unit == "%" {
        let fraction = value / 100.0;
        let note = format!("{} % becomes {}", plain(value), fraction_text(fraction));
        return Some((fraction, limit.claim_unit.clone(), Some(note)));
    }
    None
}

/// A number as written: no trailing ".0".
fn plain(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// A fraction with at least two decimals: 0.6 is "0.60", 0.145 is "0.145".
fn fraction_text(v: f64) -> String {
    let mut s = format!("{v:.6}");
    while s.ends_with('0') && s.len() - s.find('.').map_or(s.len(), |p| p + 1) > 2 {
        s.pop();
    }
    s
}

/// Whether the text states this option: its label, or its id with spaces,
/// appears in the text (case-insensitive). Unknown assumptions or options
/// are never stated.
fn stated(text: &str, limit: &CatalogLimit, assumption: &str, option: &str) -> bool {
    let Some(a) = limit.assumption(assumption) else {
        return false;
    };
    let Some(o) = a.options.iter().find(|o| o.id == option) else {
        return false;
    };
    let haystack = text.to_lowercase();
    [
        o.label.to_lowercase(),
        o.id.replace('_', " ").to_lowercase(),
    ]
    .iter()
    .any(|needle| !needle.is_empty() && haystack.contains(needle.as_str()))
}

/// The evidence as a character span of the text, when it is an exact substring.
pub fn locate(text: &str, evidence: &str) -> Option<Span> {
    if evidence.is_empty() {
        return None;
    }
    let byte = text.find(evidence)?;
    let start = text[..byte].chars().count();
    Some(Span {
        text: evidence.to_string(),
        start,
        end: start + evidence.chars().count(),
    })
}

/// Whether `value` is one of the numbers written in `evidence`. A decimal
/// comma is read as a point; "1,000" is also read as a thousand.
pub fn number_in(evidence: &str, value: f64) -> bool {
    numbers(evidence).into_iter().any(|n| same(n, value))
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()) || (a - b).abs() <= 1e-12
}

/// Every number written in the text, with its sign when one precedes it.
fn numbers(text: &str) -> Vec<f64> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let starts = chars[i].is_ascii_digit()
            || (chars[i] == '.' && chars.get(i + 1).is_some_and(|c| c.is_ascii_digit()));
        if !starts {
            i += 1;
            continue;
        }
        let negative = i > 0 && chars[i - 1] == '-' && (i < 2 || !chars[i - 2].is_alphanumeric());
        let begin = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let mut sep = None;
        if i + 1 < chars.len()
            && (chars[i] == '.' || chars[i] == ',')
            && chars[i + 1].is_ascii_digit()
        {
            sep = Some(chars[i]);
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
        if i + 1 < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
            let mut j = i + 1;
            if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                j += 1;
            }
            if j < chars.len() && chars[j].is_ascii_digit() {
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        let token: String = chars[begin..i].iter().collect();
        let sign = if negative { -1.0 } else { 1.0 };
        let mut candidates = vec![token.replace(',', ".")];
        if sep == Some(',') {
            let after = token.split(',').nth(1).map_or(0, str::len);
            if after == 3 {
                candidates.push(token.replace(',', ""));
            }
        }
        for c in candidates {
            if let Ok(n) = c.parse::<f64>() {
                out.push(sign * n);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_model_output;
    use crate::testing::catalog;

    fn checked(text: &str, json: &str) -> Vec<CheckedClaim> {
        check(
            &parse_model_output(json).expect("model output"),
            text,
            &catalog(),
        )
    }

    #[test]
    fn numbers_are_read_with_points_commas_signs_and_exponents() {
        assert_eq!(numbers("600 K and 300K"), vec![600.0, 300.0]);
        assert_eq!(numbers("60% or 0,70 or .5"), vec![60.0, 0.7, 0.5]);
        assert_eq!(numbers("1,000 W"), vec![1.0, 1000.0]);
        assert_eq!(
            numbers("from -5 to 1.2e3, model X-1"),
            vec![-5.0, 1200.0, 1.0]
        );
    }

    #[test]
    fn offsets_are_in_characters() {
        let span = locate("Ein Wärmepumpe, COP 14.5", "COP 14.5").expect("found");
        assert_eq!((span.start, span.end), (16, 24));
        assert_eq!(locate("abc", "x"), None);
        assert_eq!(locate("abc", ""), None);
    }

    #[test]
    fn percent_becomes_a_fraction_with_a_note() {
        let c = &checked(
            "at 60% efficiency",
            r#"{"claims":[{"limit_id":"carnot","quantity":"efficiency","value":60,"unit":"%","evidence":"60% efficiency","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(c.value, Some(0.6));
        assert_eq!(c.unit, "dimensionless");
        assert_eq!(c.notes, vec!["60 % becomes 0.60"]);
        assert_eq!(
            c.evidence,
            Some(Span {
                text: "60% efficiency".into(),
                start: 3,
                end: 17
            })
        );
    }

    #[test]
    fn a_unit_the_limit_does_not_allow_is_flagged_not_guessed() {
        let c = &checked(
            "between 327 °C and 27 °C at 40%",
            r#"{"claims":[{"limit_id":"carnot","quantity":"efficiency","value":40,"unit":"%","evidence":"40%","constraints":{},"parameters":{"Th":{"value":327,"unit":"°C","evidence":"327 °C"}}}],"missing":[]}"#,
        )[0];
        assert!(c.flags.iter().any(|f| f == UNIT_NOT_UNDERSTOOD));
        assert_eq!(c.parameters["Th"].value, None);
        assert_eq!(c.not_evaluable_reason(), Some(UNIT_NOT_UNDERSTOOD));
        let d = &checked(
            "efficiency of 0.4 W",
            r#"{"claims":[{"limit_id":"carnot","quantity":"efficiency","value":0.4,"unit":"W","evidence":"0.4 W","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(d.value, None);
        assert_eq!(d.not_evaluable_reason(), Some(UNIT_NOT_UNDERSTOOD));
    }

    #[test]
    fn hot_and_cold_are_never_reordered() {
        let c = &checked(
            "between 280 K (hot) and 300 K (cold) at 5%",
            r#"{"claims":[{"limit_id":"carnot","quantity":"efficiency","value":5,"unit":"%","evidence":"5%","constraints":{},"parameters":{"Th":{"value":280,"unit":"K","evidence":"280 K"},"Tc":{"value":300,"unit":"K","evidence":"300 K"}}}],"missing":[]}"#,
        )[0];
        assert_eq!(c.parameters["Th"].value, Some(280.0));
        assert_eq!(c.parameters["Tc"].value, Some(300.0));
        assert!(c.evaluable());
    }

    #[test]
    fn a_constraint_is_kept_only_when_the_text_states_it() {
        let json = |mode: &str| {
            format!(
                r#"{{"claims":[{{"limit_id":"cop","quantity":"COP","value":14.5,"unit":"dimensionless","evidence":"COP of 14.5","constraints":{{"mode":"{mode}"}},"parameters":{{}}}}],"missing":[]}}"#
            )
        };
        let kept = &checked("a cooling COP of 14.5", &json("cooling"))[0];
        assert_eq!(
            kept.constraints.get("mode").map(String::as_str),
            Some("cooling")
        );
        let dropped = &checked("a heat pump COP of 14.5", &json("heating"))[0];
        assert!(dropped.constraints.is_empty());
        assert!(dropped.flags.iter().any(|f| f == CONSTRAINT_NOT_STATED));
        let unknown = &checked("a cooling COP of 14.5", &json("freezing"))[0];
        assert!(unknown.constraints.is_empty());
    }

    #[test]
    fn an_unknown_limit_is_flagged_and_not_evaluated() {
        let c = &checked(
            "a COP of 14.5",
            r#"{"claims":[{"limit_id":null,"quantity":"COP","value":14.5,"unit":"dimensionless","evidence":"COP of 14.5","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(c.not_evaluable_reason(), Some(LIMIT_NOT_RECOGNIZED));
    }

    #[test]
    fn fractions_print_with_at_least_two_decimals() {
        assert_eq!(fraction_text(0.6), "0.60");
        assert_eq!(fraction_text(0.145), "0.145");
        assert_eq!(fraction_text(1.0), "1.00");
        assert_eq!(plain(60.0), "60");
        assert_eq!(plain(14.5), "14.5");
    }
}
