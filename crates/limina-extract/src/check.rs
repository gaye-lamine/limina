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
    let claims = output
        .claims
        .iter()
        .map(|c| {
            let (written, unit_notes) = units_as_written(c, text);
            let (resolved, note) = resolve_built_in(&written, text, catalog);
            let mut checked = check_claim(&resolved, text, catalog);
            checked.notes.extend(unit_notes);
            checked.notes.extend(note);
            checked
        })
        .collect();
    let mut claims = attach_parameter_claims(claims, catalog);
    for c in &mut claims {
        fill_parameters_from_sentence(c, text, catalog);
        fill_body_constants(c, text, catalog);
        fill_options_from_sentence(c, text, catalog);
    }
    merge_quotient_claims(claims, text)
}

/// Note prefix of a claim whose built-in law was chosen by the deterministic matcher.
pub const MATCHED_BUILT_IN: &str = "built-in law matched by quantity and dimension:";

/// The claim with the built-in law the deterministic matcher chooses; the
/// model's `limit_id` is only a hint (see [`crate::matcher`]). A note says
/// when the matcher chose a law other than the model's, or one the model
/// did not give.
fn resolve_built_in(
    claim: &ModelClaim,
    text: &str,
    catalog: &Catalog,
) -> (ModelClaim, Option<String>) {
    let Some(m) = crate::matcher::match_built_in(claim, text, catalog) else {
        return (claim.clone(), None);
    };
    if claim.limit_id.as_deref() == Some(m.id.as_str()) {
        return (claim.clone(), None);
    }
    let why = match m.by {
        crate::matcher::MatchedBy::Keyword(k) => format!("keyword \"{k}\""),
        crate::matcher::MatchedBy::Hint => "model hint".to_string(),
    };
    let hint = claim.limit_id.as_deref().unwrap_or("none");
    let note = format!("{MATCHED_BUILT_IN} {} ({why}; model hint: {hint})", m.id);
    let mut resolved = claim.clone();
    resolved.limit_id = Some(m.id);
    (resolved, Some(note))
}

/// The sentence of `text` around a character span: from just after the
/// previous sentence end (". ", "! ", "? " or a blank line) to the next one.
/// A single line break is a wrapped line (text read from an image), not an end.
pub(crate) fn sentence_around(text: &str, span: &Span) -> String {
    let chars: Vec<char> = text.chars().collect();
    let ends = |i: usize| {
        matches!(chars.get(i), Some('.' | '!' | '?'))
            && chars.get(i + 1).is_none_or(|c| c.is_whitespace())
    };
    // A blank line: this line break and the next non-space character is another line break.
    let blank = |i: usize| {
        chars.get(i) == Some(&'\n')
            && chars[i + 1..]
                .iter()
                .find(|c| !matches!(c, ' ' | '\t' | '\r'))
                .is_some_and(|c| *c == '\n')
    };
    let blank_before = |i: usize| {
        chars.get(i) == Some(&'\n')
            && chars[..i]
                .iter()
                .rev()
                .find(|c| !matches!(c, ' ' | '\t' | '\r'))
                .is_some_and(|c| *c == '\n')
    };
    let mut a = span.start.min(chars.len());
    while a > 0 && !blank_before(a - 1) && !ends(a - 1) {
        a -= 1;
    }
    let mut b = span.end.min(chars.len());
    while b < chars.len() && !blank(b) && !ends(b) {
        b += 1;
    }
    chars[a..(b + 1).min(chars.len())].iter().collect()
}

/// Every number of `sentence` immediately followed by a unit symbol the
/// core parses, with the quotation "number unit" as written.
pub(crate) fn numbers_with_units(sentence: &str) -> Vec<(f64, String, String)> {
    numbers_with_units_at(sentence)
        .into_iter()
        .map(|(v, u, q, _)| (v, u, q))
        .collect()
}

/// Like [`numbers_with_units`], with the character offset of each number
/// in `sentence`. The number and its unit may be separated by any
/// whitespace, a line break included (a wrapped line).
pub(crate) fn numbers_with_units_at(sentence: &str) -> Vec<(f64, String, String, usize)> {
    let chars: Vec<char> = sentence.chars().collect();
    let mut out = Vec::new();
    for (value, start, end) in scan_numbers(sentence) {
        // The words right after the number: one, else two ("kg m/s").
        let rest: String = chars[end.min(chars.len())..].iter().collect();
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let words: Vec<&str> = rest.split_whitespace().take(2).collect();
        let trim = |s: &str| s.trim_end_matches(['.', ',', ';', ':', ')']).to_string();
        for take in [1usize, 2] {
            let Some(unit_words) = words.get(..take) else {
                continue;
            };
            let unit = trim(&unit_words.join(" "));
            if !unit.is_empty() && limina_core::parse_unit(&unit).is_ok() {
                let number: String = chars[start..end].iter().collect();
                out.push((value, unit.clone(), format!("{number} {unit}"), start));
                break;
            }
        }
    }
    out
}

/// The only number of the sentence around `[start, end)` (characters of
/// `text`) written in one of `units`, with its unit as written; None when
/// there is none or more than one. Nothing is computed or converted.
pub fn unique_number_in_units(
    text: &str,
    start: usize,
    end: usize,
    units: &[String],
) -> Option<(f64, String)> {
    let span = Span {
        text: String::new(),
        start,
        end,
    };
    let found: Vec<(f64, String, String)> = numbers_with_units(&sentence_around(text, &span))
        .into_iter()
        .filter(|(_, u, _)| units.iter().any(|pu| same_unit(pu, u)))
        .collect();
    match found.as_slice() {
        [(v, u, _)] => Some((*v, u.clone())),
        _ => None,
    }
}

/// Words that name a parameter in a sentence, by parameter name. Only
/// used to choose between several numbers of the same unit.
const PARAMETER_CUES: &[(&str, &[&str])] = &[
    (
        "Th",
        &["hot", "source", "high temperature", "hot side", "warm"],
    ),
    (
        "Tc",
        &[
            "cold",
            "sink",
            "low temperature",
            "cold side",
            "ground",
            "outside",
            "outdoor",
        ],
    ),
    ("wavelength", &["wavelength", "light"]),
    ("aperture", &["aperture", "na"]),
    ("dp", &["momentum"]),
    ("T", &["temperature"]),
    (
        "P_in",
        &["input", "consumes", "consumed", "draws", "supplied", "fed"],
    ),
    ("D", &["diameter", "aperture", "mirror", "lens"]),
    (
        "dm",
        &["converting", "convert", "converts", "converted", "mass"],
    ),
];

/// Whether the words between the previous number of the sentence and the
/// number at `at` name the parameter `name` (whole words, case ignored).
fn names_parameter(
    sentence: &str,
    at: usize,
    name: &str,
    found: &[(f64, String, String, usize)],
) -> bool {
    let Some((_, cues)) = PARAMETER_CUES.iter().find(|(n, _)| *n == name) else {
        return false;
    };
    let from = found
        .iter()
        .map(|(_, _, q, s)| s + q.chars().count())
        .filter(|e| *e <= at)
        .max()
        .unwrap_or(0);
    let before: String = sentence
        .chars()
        .skip(from)
        .take(at.saturating_sub(from))
        .collect();
    let words = format!(
        " {} ",
        before
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    );
    cues.iter().any(|c| words.contains(&format!(" {c} ")))
}

/// The character range `[start, end)` of the sentence of `text` around `span`.
fn sentence_bounds(text: &str, span: &Span) -> (usize, usize) {
    let sentence = sentence_around(text, span);
    let n = sentence.chars().count();
    let chars: Vec<char> = text.chars().collect();
    // The sentence contains the span; search its start at or before the span.
    let first = span.start.min(chars.len());
    for start in (0..=first).rev() {
        if start + n <= chars.len() && chars[start..start + n].iter().copied().eq(sentence.chars())
        {
            return (start, start + n);
        }
    }
    (0, chars.len())
}

/// Note prefix of a value whose unit was read from the text.
pub const UNIT_FROM_TEXT: &str = "unit read from the text:";

/// The unit written right after `value` in the sentence of `evidence`,
/// when exactly one written unit follows that number there.
fn written_unit(text: &str, evidence: &str, value: f64) -> Option<String> {
    let span = locate(text, evidence)?;
    let sentence = sentence_around(text, &span);
    let units: Vec<String> = numbers_with_units(&sentence)
        .into_iter()
        .filter(|(v, _, _)| same(*v, value))
        .map(|(_, u, _)| u)
        .collect();
    match units.as_slice() {
        [u] => Some(u.clone()),
        [u, rest @ ..] if rest.iter().all(|x| same_unit(x, u)) => Some(u.clone()),
        _ => None,
    }
}

/// The claim with each unit as the text writes it next to its number. A
/// model may drop a prefix ("10 Mbit/s" read as bit/s) or name another
/// unit ("1e-5 rad" read as m); the text wins, with a note. Nothing is
/// converted here and a number without a written unit keeps the model's.
fn units_as_written(claim: &ModelClaim, text: &str) -> (ModelClaim, Vec<String>) {
    let mut out = claim.clone();
    let mut notes = Vec::new();
    if let Some(u) = written_unit(text, &claim.evidence, claim.value) {
        if !same_unit(&u, &claim.unit) {
            notes.push(format!(
                "{UNIT_FROM_TEXT} {} {u} (the reading said {})",
                short_number(claim.value),
                claim.unit
            ));
            out.unit = u;
        }
    }
    for (name, p) in out.parameters.iter_mut() {
        if let Some(u) = written_unit(text, &p.evidence, p.value) {
            if !same_unit(&u, &p.unit) {
                notes.push(format!(
                    "{UNIT_FROM_TEXT} {name} = {} {u} (the reading said {})",
                    short_number(p.value),
                    p.unit
                ));
                p.unit = u;
            }
        }
    }
    (out, notes)
}

/// Note prefix of an assumption read from the claim's own sentence.
pub const OPTION_FROM_TEXT: &str = "assumption stated in the text:";

/// An assumption of the claim's law that the model left out, when its
/// sentence states exactly one of its options by that option's cues
/// ([`crate::matcher::stated_option`]). Two options stated, or none: left out.
fn fill_options_from_sentence(claim: &mut CheckedClaim, text: &str, catalog: &Catalog) {
    let (Some(id), Some(span)) = (claim.limit_id.clone(), claim.evidence.clone()) else {
        return;
    };
    let Some(limit) = catalog.limit(&id) else {
        return;
    };
    let sentence = sentence_around(text, &span);
    for a in &limit.assumptions {
        if claim.constraints.contains_key(&a.id) || a.options.len() < 2 {
            continue;
        }
        let ids: Vec<&str> = a.options.iter().map(|o| o.id.as_str()).collect();
        if let Some(option) = crate::matcher::stated_option(&sentence, &ids) {
            claim.constraints.insert(a.id.clone(), option.to_string());
            let label = a
                .options
                .iter()
                .find(|o| o.id == option)
                .map_or(option, |o| o.label.as_str());
            claim.notes.push(format!("{OPTION_FROM_TEXT} {label}"));
        }
    }
}

/// A body named in a sentence, with its nominal gravitational parameter
/// GM (m^3/s^2) and its nominal equatorial radius (m), from IAU 2015
/// Resolution B3. Only bodies with IAU nominal values are listed.
const BODIES: &[(&str, &[&str], f64, f64)] = &[
    ("Earth", &["earth", "earth s"], 3.986_004e14, 6.378_1e6),
    (
        "the Sun",
        &["sun", "sun s", "solar surface"],
        1.327_124_4e20,
        6.957e8,
    ),
];

/// Note prefix of parameters taken from a named body's nominal values.
pub const BODY_CONSTANTS: &str = "values of a named body (IAU 2015 nominal):";

/// GM and r of a law left out by the text, when its sentence names exactly
/// one body of [`BODIES`] and says the claim starts from its surface: the
/// body's nominal GM and equatorial radius. Nothing else is filled.
fn fill_body_constants(claim: &mut CheckedClaim, text: &str, catalog: &Catalog) {
    let (Some(id), Some(span)) = (claim.limit_id.clone(), claim.evidence.clone()) else {
        return;
    };
    let Some(limit) = catalog.limit(&id) else {
        return;
    };
    let wants = |n: &str| limit.parameters.iter().any(|p| p.name == n);
    let missing = |n: &str| claim.parameters.get(n).is_none_or(|p| p.value.is_none());
    if !(wants("GM") && wants("r") && missing("GM") && missing("r")) {
        return;
    }
    let sentence = sentence_around(text, &span).to_lowercase();
    let w = format!(
        " {} ",
        sentence
            .split(|c: char| !c.is_alphanumeric())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    );
    if !w.contains(" surface ") {
        return;
    }
    let named: Vec<&(&str, &[&str], f64, f64)> = BODIES
        .iter()
        .filter(|(_, names, _, _)| names.iter().any(|n| w.contains(&format!(" {n} "))))
        .collect();
    let [(name, _, gm, radius)] = named.as_slice() else {
        return;
    };
    for (param, value, unit) in [("GM", *gm, "m^3/s^2"), ("r", *radius, "m")] {
        claim.parameters.insert(
            param.to_string(),
            CheckedParameter {
                value: Some(value),
                raw_value: value,
                unit: unit.to_string(),
                evidence: None,
                flags: Vec::new(),
            },
        );
    }
    claim.notes.push(format!(
        "{BODY_CONSTANTS} {name}, GM = {} m^3/s^2, r = {} m (equatorial radius at the surface)",
        short_number(*gm),
        short_number(*radius)
    ));
}

/// Note added to a claim whose parameter was read from its own sentence.
pub const READ_FROM_SENTENCE: &str = "read from the same sentence by its unit";

/// A declared parameter the model left out, when its sentence states exactly
/// one number in a unit of that parameter (other than the claim's own
/// number): the number and its quotation are taken from the text. Nothing
/// is computed or converted; an ambiguous sentence is left as it is.
fn fill_parameters_from_sentence(claim: &mut CheckedClaim, text: &str, catalog: &Catalog) {
    let (Some(id), Some(span)) = (claim.limit_id.clone(), claim.evidence.clone()) else {
        return;
    };
    let Some(limit) = catalog.limit(&id) else {
        return;
    };
    let (s_start, s_end) = sentence_bounds(text, &span);
    let sentence: String = text.chars().skip(s_start).take(s_end - s_start).collect();
    let found = numbers_with_units_at(&sentence);
    // Numbers already used: the claim's own and the traced parameters'.
    let mut used: Vec<f64> = claim
        .parameters
        .values()
        .filter_map(|p| p.value)
        .chain(std::iter::once(claim.raw_value))
        .collect();
    for p in &limit.parameters {
        if claim
            .parameters
            .get(&p.name)
            .is_some_and(|x| x.value.is_some())
        {
            continue;
        }
        let in_unit: Vec<&(f64, String, String, usize)> = found
            .iter()
            .filter(|(_, u, _, _)| p.units.iter().any(|pu| same_unit(pu, u)))
            .collect();
        let free = in_unit
            .iter()
            .copied()
            .filter(|(v, _, _, _)| !used.iter().any(|x| same(*x, *v)));
        // One number in the unit: that one. Several (two temperatures): only the one
        // the words before it name ("cold reservoir at 273 K"); never by elimination,
        // so a hot and a cold temperature are never swapped.
        let chosen: Vec<&(f64, String, String, usize)> = if in_unit.len() > 1 {
            free.filter(|(_, _, _, at)| names_parameter(&sentence, *at, &p.name, &found))
                .collect()
        } else {
            free.collect()
        };
        let [(value, unit, quote, at)] = chosen.as_slice() else {
            continue;
        };
        let Some(inner) = locate(&sentence.chars().skip(*at).collect::<String>(), quote) else {
            continue;
        };
        let evidence = Span {
            text: inner.text,
            start: s_start + at + inner.start,
            end: s_start + at + inner.end,
        };
        used.push(*value);
        claim.parameters.insert(
            p.name.clone(),
            CheckedParameter {
                value: Some(*value),
                raw_value: *value,
                unit: unit.clone(),
                evidence: Some(evidence),
                flags: Vec::new(),
            },
        );
        claim.notes.push(format!(
            "{} = {} {} {READ_FROM_SENTENCE}",
            p.name,
            short_number(*value),
            unit
        ));
    }
    if !claim
        .parameters
        .values()
        .any(|p| p.flags.iter().any(|f| f == UNIT_NOT_UNDERSTOOD))
    {
        claim
            .flags
            .retain(|f| f != UNIT_NOT_UNDERSTOOD || claim.value.is_none());
    }
}

/// Note prefix of a claim made of two readings of one sentence: an amount
/// per unit of another amount ("5 kWh of energy in a mass of 1 kg").
pub const MERGED_QUOTIENT: &str = "two readings of one sentence merged into";

const ENERGY: [i8; 7] = [2, 1, -2, 0, 0, 0, 0];
const MASS: [i8; 7] = [0, 1, 0, 0, 0, 0, 0];
const VOLUME: [i8; 7] = [3, 0, 0, 0, 0, 0, 0];

fn dimension_of(unit: &str) -> Option<[i8; 7]> {
    limina_core::parse_unit(unit)
        .ok()
        .map(|u| u.dimension.exponents())
}

/// Two claims without a known limit, both traceable to the same sentence,
/// one an energy and the other a mass (or a volume), are one claim: the
/// specific energy (or energy density) E / m in the unit "E-unit/m-unit".
/// The note shows both readings. Nothing happens in any other case.
fn merge_quotient_claims(mut claims: Vec<CheckedClaim>, text: &str) -> Vec<CheckedClaim> {
    let unknown = |c: &CheckedClaim| c.flags.iter().any(|f| f == LIMIT_NOT_RECOGNIZED);
    let mut drop = vec![false; claims.len()];
    for i in 0..claims.len() {
        if drop[i] || !unknown(&claims[i]) || dimension_of(&claims[i].raw_unit) != Some(ENERGY) {
            continue;
        }
        let Some(si) = claims[i].evidence.clone() else {
            continue;
        };
        let sentence = sentence_around(text, &si);
        let partner = (0..claims.len()).find(|&j| {
            j != i
                && !drop[j]
                && unknown(&claims[j])
                && matches!(dimension_of(&claims[j].raw_unit), Some(MASS) | Some(VOLUME))
                && claims[j].raw_value != 0.0
                && claims[j]
                    .evidence
                    .as_ref()
                    .is_some_and(|e| sentence_around(text, e) == sentence)
        });
        let Some(j) = partner else {
            continue;
        };
        let (e, m) = (claims[i].clone(), claims[j].clone());
        let per_mass = dimension_of(&m.raw_unit) == Some(MASS);
        let unit = format!("{}/{}", e.raw_unit, m.raw_unit);
        let value = e.raw_value / m.raw_value;
        let c = &mut claims[i];
        c.quantity = if per_mass {
            "specific energy"
        } else {
            "energy density"
        }
        .to_string();
        c.raw_value = value;
        c.raw_unit = unit.clone();
        c.unit = unit.clone();
        c.notes.push(format!(
            "{MERGED_QUOTIENT} {}: {} {} in {} {} is {} {unit}",
            c.quantity,
            short_number(e.raw_value),
            e.raw_unit,
            short_number(m.raw_value),
            m.raw_unit,
            short_number(value)
        ));
        drop[j] = true;
    }
    let mut k = 0;
    claims.retain(|_| {
        k += 1;
        !drop[k - 1]
    });
    claims
}

/// A number for people: at most 10 significant digits, without trailing
/// zeros; scientific notation below 1e-3 and from 1e6. The last-bit noise of
/// a decimal reading ("1.0000000000000001e-24") is not shown. Display only:
/// values sent to the engine are never rounded.
pub fn short_number(v: f64) -> String {
    if !v.is_finite() {
        return format!("{v}");
    }
    if v != 0.0 && (v.abs() < 1e-3 || v.abs() >= 1e6) {
        let s = format!("{v:.9e}");
        let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mantissa = if mantissa.contains('.') {
            mantissa.trim_end_matches('0').trim_end_matches('.')
        } else {
            mantissa
        };
        return format!("{mantissa}e{exp}");
    }
    let digits = (9 - v.abs().log10().floor().max(0.0) as i32).max(0) as usize;
    let s = format!("{v:.digits$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Whether two unit symbols are the same unit: written alike, or parsed by
/// the core to the same dimension and the same factor (`kg·m/s` and
/// `kg*m/s`). No conversion is ever made. The core counts bit, rad and sr
/// as dimensionless, so a unit naming one of them is never the same as a
/// unit that does not (Mbit/s is not MHz, rad is not a plain number).
pub fn same_unit(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if named_dimensionless(a) != named_dimensionless(b) {
        return false;
    }
    match (limina_core::parse_unit(a), limina_core::parse_unit(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// The dimensionless units a symbol names (bit, rad, sr), with any prefix.
fn named_dimensionless(unit: &str) -> [bool; 3] {
    let words: Vec<&str> = unit
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| !w.is_empty())
        .collect();
    let names = |n: &str| words.iter().any(|w| w.ends_with(n));
    [names("bit"), names("rad"), names("sr")]
}

/// Note added to a claim that received a parameter read as a separate claim.
pub const ATTACHED_AS_PARAMETER: &str = "read as a separate claim, attached as parameter";

/// A claim whose unit is that of a parameter of its own limit, not of the
/// claimed quantity, is a parameter written by the model as a claim. When
/// exactly one other claim of the same limit lacks that parameter, its
/// reading becomes that parameter, with the same evidence rules; the
/// separate claim is dropped and the host claim notes it. Nothing else
/// changes: an ambiguous case (no host or several hosts) is left as it is.
fn attach_parameter_claims(mut claims: Vec<CheckedClaim>, catalog: &Catalog) -> Vec<CheckedClaim> {
    let mut drop = vec![false; claims.len()];
    for i in 0..claims.len() {
        let c = &claims[i];
        let (Some(id), Some(evidence)) = (c.limit_id.clone(), c.evidence.clone()) else {
            continue;
        };
        if !c.flags.iter().any(|f| f == UNIT_NOT_UNDERSTOOD)
            || !number_in(&evidence.text, c.raw_value)
        {
            continue;
        }
        let Some(limit) = catalog.limit(&id) else {
            continue;
        };
        let Some(param) = limit
            .parameters
            .iter()
            .find(|p| p.units.iter().any(|u| same_unit(u, &c.raw_unit)))
        else {
            continue;
        };
        let hosts: Vec<usize> = (0..claims.len())
            .filter(|&j| {
                j != i
                    && !drop[j]
                    && claims[j].limit_id.as_deref() == Some(id.as_str())
                    && claims[j]
                        .parameters
                        .get(&param.name)
                        .is_none_or(|p| p.value.is_none())
            })
            .collect();
        let [host] = hosts.as_slice() else {
            continue;
        };
        let reading = CheckedParameter {
            value: Some(c.raw_value),
            raw_value: c.raw_value,
            unit: c.raw_unit.clone(),
            evidence: Some(evidence),
            flags: Vec::new(),
        };
        let note = format!(
            "{} ({}) {ATTACHED_AS_PARAMETER} {}",
            c.quantity,
            plain(c.raw_value),
            param.name
        );
        let host = &mut claims[*host];
        host.parameters.insert(param.name.clone(), reading);
        host.notes.push(note);
        if !host
            .parameters
            .values()
            .any(|p| p.flags.iter().any(|f| f == UNIT_NOT_UNDERSTOOD))
        {
            host.flags.retain(|f| f != UNIT_NOT_UNDERSTOOD);
        }
        drop[i] = true;
    }
    let mut k = 0;
    claims.retain(|_| {
        k += 1;
        !drop[k - 1]
    });
    claims
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
    let allowed =
        |spec: &crate::catalog::CatalogParameter| spec.units.iter().any(|u| same_unit(u, &p.unit));
    match declared {
        None if limit.is_some() => push_once(&mut flags, PARAMETER_NOT_DECLARED),
        Some(spec) if !allowed(spec) => push_once(&mut flags, UNIT_NOT_UNDERSTOOD),
        _ => {}
    }
    let ok = traced && declared.is_some_and(allowed);
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
    if unit == limit.claim_unit
        || (dimensionless && unit.is_empty())
        || (!dimensionless && same_unit(unit, &limit.claim_unit))
    {
        return Some((value, limit.claim_unit.clone(), None));
    }
    if dimensionless && unit == "%" {
        let fraction = value / 100.0;
        let note = format!("{} % becomes {}", plain(value), fraction_text(fraction));
        return Some((fraction, limit.claim_unit.clone(), Some(note)));
    }
    // Another SI unit of the same dimension (nm for m, kWh for J): exact SI
    // factors, never an offset (Celsius is not an SI unit and never parses),
    // with a visible note. Anything else is not understood.
    if dimensionless {
        return None;
    }
    let (from, to) = (
        limina_core::parse_unit(unit).ok()?,
        limina_core::parse_unit(&limit.claim_unit).ok()?,
    );
    if from.dimension != to.dimension {
        return None;
    }
    let converted = value * from.factor_to_si / to.factor_to_si;
    if !converted.is_finite() {
        return None;
    }
    let note = format!(
        "{} {unit} becomes {} {}",
        short_number(value),
        short_number(converted),
        limit.claim_unit
    );
    Some((converted, limit.claim_unit.clone(), Some(note)))
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

/// The evidence as a character span of the text, when it is a substring of
/// it. Runs of whitespace match any run of whitespace, so a quotation with
/// a space is found where the text wraps the line; the span then holds the
/// text as written there. Nothing else is relaxed.
pub fn locate(text: &str, evidence: &str) -> Option<Span> {
    if evidence.trim().is_empty() {
        return None;
    }
    if let Some(byte) = text.find(evidence) {
        let start = text[..byte].chars().count();
        return Some(Span {
            text: evidence.to_string(),
            start,
            end: start + evidence.chars().count(),
        });
    }
    let chars: Vec<char> = text.chars().collect();
    let want: Vec<char> = evidence.trim().chars().collect();
    'outer: for start in 0..chars.len() {
        let (mut i, mut j) = (start, 0usize);
        while j < want.len() {
            if want[j].is_whitespace() {
                if i >= chars.len() || !chars[i].is_whitespace() {
                    continue 'outer;
                }
                while j < want.len() && want[j].is_whitespace() {
                    j += 1;
                }
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
            } else {
                if i >= chars.len() || chars[i] != want[j] {
                    continue 'outer;
                }
                i += 1;
                j += 1;
            }
        }
        return Some(Span {
            text: chars[start..i].iter().collect(),
            start,
            end: i,
        });
    }
    None
}

/// Whether `value` is one of the numbers written in `evidence`. A decimal
/// comma is read as a point; "1,000" is also read as a thousand.
pub fn number_in(evidence: &str, value: f64) -> bool {
    numbers(evidence).into_iter().any(|n| same(n, value))
}

/// Equal up to a relative 1e-9. No absolute tolerance: with one, every
/// very small number (1e-12, 1e-24, 1e-36) would match every other.
fn same(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs())
}

/// Every number written in the text, with its sign when one precedes it.
fn numbers(text: &str) -> Vec<f64> {
    scan_numbers(text).into_iter().map(|(v, _, _)| v).collect()
}

/// Every number of the text with its character span `[start, end)`.
fn scan_numbers(text: &str) -> Vec<(f64, usize, usize)> {
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
        // "a × 10⁻¹⁰", "a x 10^-10", "a·10^(-10)", "a*10**-10", and a bare
        // "10⁻¹⁰" or "10^-10": the whole number only. Its mantissa alone is
        // not a number of the text, so a model that drops the power of ten
        // is not traceable.
        if let Some((times, end)) = power_of_ten(&chars, i) {
            let mantissa = token.replace(',', ".").parse::<f64>().ok();
            let bare = !times && token == "10";
            if let (Some(m), Some(exp)) = (mantissa, ten_exponent(&chars, i, times)) {
                // Parsed from its decimal form, so "1 × 10⁻²⁴" is exactly 1e-24.
                let mantissa_text = if bare {
                    "1".to_string()
                } else {
                    token.replace(',', ".")
                };
                let value = format!("{mantissa_text}e{exp}")
                    .parse::<f64>()
                    .unwrap_or(m * 10f64.powi(exp));
                let _ = m;
                out.push((sign * value, begin - usize::from(negative), end));
                i = end;
                continue;
            }
        }
        let mut candidates = vec![token.replace(',', ".")];
        if sep == Some(',') {
            let after = token.split(',').nth(1).map_or(0, str::len);
            if after == 3 {
                candidates.push(token.replace(',', ""));
            }
        }
        for c in candidates {
            if let Ok(n) = c.parse::<f64>() {
                out.push((sign * n, begin - usize::from(negative), i));
            }
        }
    }
    out
}

fn superscript_digit(c: char) -> Option<u32> {
    "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|s| s == c).map(|p| p as u32)
}

fn is_minus(c: char) -> bool {
    c == '-' || c == '−' || c == '⁻' || c == '–'
}

/// After the mantissa ending at `i`: an optional multiplication sign and
/// "10" followed by an exponent (when `times`), or directly an exponent
/// after a bare "10". Returns whether a multiplication sign was seen and the
/// index after the whole number.
fn power_of_ten(chars: &[char], i: usize) -> Option<(bool, usize)> {
    let mut j = i;
    let skip = |j: &mut usize| {
        while *j < chars.len()
            && (chars[*j] == ' ' || chars[*j] == '\u{a0}' || chars[*j] == '\u{202f}')
        {
            *j += 1;
        }
    };
    skip(&mut j);
    let times = j < chars.len() && matches!(chars[j], '×' | 'x' | 'X' | '*' | '·' | '⋅' | '∙');
    if times {
        j += 1;
        skip(&mut j);
        if chars.get(j) != Some(&'1') || chars.get(j + 1) != Some(&'0') {
            return None;
        }
        j += 2;
        if chars.get(j).is_some_and(|c| c.is_ascii_digit()) {
            return None;
        }
    } else if j != i {
        return None;
    } else {
        // A bare "10": the exponent must follow immediately.
        let before: String = chars[i.saturating_sub(2)..i].iter().collect();
        if before != "10" || (i >= 3 && (chars[i - 3].is_ascii_digit() || chars[i - 3] == '.')) {
            return None;
        }
    }
    let end = exponent_end(chars, j)?;
    Some((times, end))
}

/// The index after an exponent starting at `j`: `^n`, `^-n`, `^(-n)`,
/// `**n`, or superscript digits with an optional superscript minus.
fn exponent_end(chars: &[char], j: usize) -> Option<usize> {
    let mut k = j;
    if chars.get(k) == Some(&'^') || (chars.get(k) == Some(&'*') && chars.get(k + 1) == Some(&'*'))
    {
        k += if chars[k] == '^' { 1 } else { 2 };
        let paren = chars.get(k) == Some(&'(');
        if paren {
            k += 1;
        }
        if chars.get(k).is_some_and(|c| is_minus(*c) || *c == '+') {
            k += 1;
        }
        let digits = k;
        while chars.get(k).is_some_and(|c| c.is_ascii_digit()) {
            k += 1;
        }
        if k == digits {
            return None;
        }
        if paren {
            if chars.get(k) != Some(&')') {
                return None;
            }
            k += 1;
        }
        return Some(k);
    }
    // A typographic minus (U+2212) right after "10", as in text pasted from
    // a formula: "4 × 10−7". The ASCII hyphen is not accepted here, so a
    // range such as "10-20" is never read as a power of ten.
    if chars.get(k) == Some(&'−') && chars.get(k + 1).is_some_and(|c| c.is_ascii_digit()) {
        k += 1;
        while chars.get(k).is_some_and(|c| c.is_ascii_digit()) {
            k += 1;
        }
        return Some(k);
    }
    if chars.get(k).is_some_and(|c| *c == '⁻' || *c == '⁺') {
        k += 1;
    }
    let digits = k;
    while chars
        .get(k)
        .is_some_and(|c| superscript_digit(*c).is_some())
    {
        k += 1;
    }
    (k > digits).then_some(k)
}

/// The exponent of the power of ten after the mantissa ending at `i`.
fn ten_exponent(chars: &[char], i: usize, times: bool) -> Option<i32> {
    let (_, end) = power_of_ten(chars, i)?;
    let mut start = i;
    if times {
        // Skip to just after "10".
        let mut j = i;
        while j + 1 < end && !(chars[j] == '1' && chars[j + 1] == '0') {
            j += 1;
        }
        start = j + 2;
    }
    let text: String = chars[start..end]
        .iter()
        .filter_map(|c| match c {
            '^' | '(' | ')' | '+' | '⁺' | '*' => None,
            c if is_minus(*c) => Some('-'),
            c => Some(
                superscript_digit(*c)
                    .and_then(|d| char::from_digit(d, 10))
                    .unwrap_or(*c),
            ),
        })
        .collect();
    let exp = text.parse::<i32>().ok()?;
    (-400..=400).contains(&exp).then_some(exp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_model_output;

    fn heisenberg_catalog() -> Catalog {
        Catalog {
            limits: vec![CatalogLimit {
                id: "heisenberg".into(),
                title: "Heisenberg position-momentum relation".into(),
                formula: "hbar/(2*dp)".into(),
                claim_unit: "m".into(),
                parameters: vec![crate::catalog::CatalogParameter {
                    name: "dp".into(),
                    units: vec!["kg*m/s".into()],
                }],
                assumptions: vec![],
            }],
        }
    }

    const HEISENBERG: &str = "A particle has a position uncertainty of 1 × 10⁻¹⁰ meters and a momentum uncertainty of 1 × 10⁻²⁴ kg·m/s.";

    #[test]
    fn a_parameter_written_as_its_own_claim_is_attached_to_the_claim_that_needs_it() {
        let json = r#"{"claims":[
            {"limit_id":"heisenberg","quantity":"position uncertainty","value":1e-10,"unit":"m","evidence":"1 × 10⁻¹⁰ meters","constraints":{},"parameters":{}},
            {"limit_id":"heisenberg","quantity":"momentum uncertainty","value":1e-24,"unit":"kg·m/s","evidence":"1 × 10⁻²⁴ kg·m/s","constraints":{},"parameters":{}}
        ],"missing":[]}"#;
        let out = check(
            &parse_model_output(json).expect("json"),
            HEISENBERG,
            &heisenberg_catalog(),
        );
        assert_eq!(out.len(), 1, "{out:?}");
        let c = &out[0];
        assert_eq!(c.value, Some(1e-10));
        let dp = &c.parameters["dp"];
        assert!(dp.value.is_some_and(|v| (v / 1e-24 - 1.0).abs() < 1e-12));
        assert_eq!(
            dp.evidence.as_ref().map(|e| e.text.as_str()),
            Some("1 × 10⁻²⁴ kg·m/s")
        );
        assert!(c.evaluable());
        assert!(
            c.notes.iter().any(|n| n.contains(ATTACHED_AS_PARAMETER)),
            "{:?}",
            c.notes
        );
    }

    /// A live answer: the model computed the product dx*dp (1e-36), which the
    /// text does not state, and quoted the whole sentence as evidence.
    #[test]
    fn a_value_the_model_computed_is_not_traceable_even_when_very_small() {
        let text = "The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.";
        let json = r#"{"claims":[
            {"limit_id":"heisenberg","quantity":"position-momentum uncertainty product","value":1e-36,"unit":"kg*m/s",
             "evidence":"The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.",
             "constraints":{},"parameters":{"dp":{"value":1e-24,"unit":"kg*m/s","evidence":"1e-24 kg*m/s"}}}
        ],"missing":[]}"#;
        let output = parse_model_output(json).expect("json");
        let out = check(&output, text, &heisenberg_catalog());
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].value, None);
        assert_eq!(out[0].evidence, None);
        assert!(
            out[0].flags.iter().any(|f| f == NOT_TRACEABLE),
            "{:?}",
            out[0].flags
        );
        assert!(!out[0].evaluable());
        // The whole answer quotes no claim value: the next model is asked and nothing is cached.
        assert!(crate::extractor::untraceable(
            &output,
            text,
            &heisenberg_catalog()
        ));
        // Small numbers are compared relatively, not with an absolute tolerance.
        assert!(!number_in(text, 1e-36));
        assert!(!number_in("1e-12", 1e-24));
        assert!(number_in("1e-24", 1e-24));
        assert!(number_in("0", 0.0));
    }

    #[test]
    fn nothing_is_attached_when_the_host_is_ambiguous_or_the_value_untraceable() {
        let two_hosts = r#"{"claims":[
            {"limit_id":"heisenberg","quantity":"a","value":1e-10,"unit":"m","evidence":"1 × 10⁻¹⁰ meters","constraints":{},"parameters":{}},
            {"limit_id":"heisenberg","quantity":"b","value":1e-10,"unit":"m","evidence":"position uncertainty of 1 × 10⁻¹⁰ meters","constraints":{},"parameters":{}},
            {"limit_id":"heisenberg","quantity":"p","value":1e-24,"unit":"kg·m/s","evidence":"1 × 10⁻²⁴ kg·m/s","constraints":{},"parameters":{}}
        ],"missing":[]}"#;
        let out = check(
            &parse_model_output(two_hosts).expect("json"),
            HEISENBERG,
            &heisenberg_catalog(),
        );
        assert_eq!(out.len(), 3);
        let misread = r#"{"claims":[
            {"limit_id":"heisenberg","quantity":"x","value":1e-10,"unit":"m","evidence":"1 × 10⁻¹⁰ meters","constraints":{},"parameters":{}},
            {"limit_id":"heisenberg","quantity":"p","value":1,"unit":"kg·m/s","evidence":"1 × 10⁻²⁴ kg·m/s","constraints":{},"parameters":{}}
        ],"missing":[]}"#;
        let out = check(
            &parse_model_output(misread).expect("json"),
            HEISENBERG,
            &heisenberg_catalog(),
        );
        assert_eq!(out.len(), 2, "a misread value is never attached");
        // The host still gets dp, read from the text itself, never the misread 1.
        let dp = out[0].parameters["dp"]
            .value
            .expect("read from the sentence");
        assert!((dp / 1e-24 - 1.0).abs() < 1e-12, "{dp}");
    }

    #[test]
    fn a_claim_in_another_si_unit_of_the_same_dimension_is_converted_with_a_note() {
        let text = "The Lumen microscope resolves details of 100 nm.";
        let catalog = Catalog {
            limits: vec![CatalogLimit {
                id: "abbe_limit".into(),
                title: "Abbe diffraction limit".into(),
                formula: "wavelength/(2*aperture)".into(),
                claim_unit: "m".into(),
                parameters: vec![],
                assumptions: vec![],
            }],
        };
        let json = r#"{"claims":[{"limit_id":"abbe_limit","quantity":"resolved size","value":100,"unit":"nm","evidence":"details of 100 nm","constraints":{},"parameters":{}}],"missing":[]}"#;
        let c = &check(&parse_model_output(json).expect("json"), text, &catalog)[0];
        assert!(
            c.value.is_some_and(|v| (v - 1e-7).abs() < 1e-20),
            "{:?}",
            c.value
        );
        assert_eq!(c.unit, "m");
        assert!(c.evaluable());
        assert!(
            c.notes.iter().any(|n| n.contains("100 nm becomes")),
            "{:?}",
            c.notes
        );
        // A wrong unit from the model: the unit written in the text wins, with a note.
        for wrong in ["s", "°C", "furlong"] {
            let j = json.replace(r#""unit":"nm""#, &format!(r#""unit":"{wrong}""#));
            let c = &check(&parse_model_output(&j).expect("json"), text, &catalog)[0];
            assert!(
                c.value.is_some_and(|v| (v - 1e-7).abs() < 1e-20)
                    && c.notes.iter().any(|n| n.starts_with(UNIT_FROM_TEXT)),
                "{wrong}: {c:?}"
            );
        }
        // Without a unit written in the text, a wrong unit is not understood.
        let bare = "The Lumen microscope resolves details of 100 with light.";
        let j = r#"{"claims":[{"limit_id":"abbe_limit","quantity":"resolved size","value":100,"unit":"furlong","evidence":"details of 100","constraints":{},"parameters":{}}],"missing":[]}"#;
        let c = &check(&parse_model_output(j).expect("json"), bare, &catalog)[0];
        assert!(
            c.value.is_none() && c.flags.iter().any(|f| f == UNIT_NOT_UNDERSTOOD),
            "{c:?}"
        );
    }

    const QSCOPE: &str = "The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.";

    #[test]
    fn a_parameter_the_model_left_out_is_read_from_the_same_sentence_by_its_unit() {
        // Seen live: the claim quotes the whole sentence and gives no dp.
        let json = r#"{"claims":[{"limit_id":"heisenberg","quantity":"position uncertainty","value":1e-12,"unit":"m","evidence":"The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.","constraints":{},"parameters":[]}],"missing":[]}"#;
        let out = check(
            &parse_model_output(json).expect("json"),
            QSCOPE,
            &heisenberg_catalog(),
        );
        assert_eq!(out.len(), 1);
        let dp = &out[0].parameters["dp"];
        assert_eq!(dp.value, Some(1e-24));
        assert_eq!(
            dp.evidence.as_ref().map(|e| e.text.as_str()),
            Some("1e-24 kg*m/s")
        );
        assert!(out[0].evaluable());
        assert!(out[0].notes.iter().any(|n| n.contains(READ_FROM_SENTENCE)));
        // Seen live: two claims of the same limit; the second becomes dp.
        let two = r#"{"claims":[
            {"limit_id":"heisenberg","quantity":"position uncertainty","value":1e-12,"unit":"m","evidence":"locates an electron within 1e-12 m","constraints":{},"parameters":{}},
            {"limit_id":"heisenberg","quantity":"momentum uncertainty","value":1e-24,"unit":"kg*m/s","evidence":"momentum is known to within 1e-24 kg*m/s","constraints":{},"parameters":{}}
        ],"missing":[]}"#;
        let out = check(
            &parse_model_output(two).expect("json"),
            QSCOPE,
            &heisenberg_catalog(),
        );
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].parameters["dp"]
            .value
            .is_some_and(|v| (v / 1e-24 - 1.0).abs() < 1e-12));
    }

    #[test]
    fn a_sentence_with_two_numbers_in_the_parameter_unit_is_left_as_it_is() {
        let text = "Within 1e-12 m, momenta of 1e-24 kg*m/s and 2e-24 kg*m/s.";
        let json = r#"{"claims":[{"limit_id":"heisenberg","quantity":"x","value":1e-12,"unit":"m","evidence":"Within 1e-12 m","constraints":{},"parameters":{}}],"missing":[]}"#;
        let out = check(
            &parse_model_output(json).expect("json"),
            text,
            &heisenberg_catalog(),
        );
        assert!(out[0].parameters.is_empty(), "ambiguous: nothing is chosen");
    }

    #[test]
    fn energy_in_a_mass_from_one_sentence_is_one_specific_energy_claim() {
        let text =
            "The Volta cell stores 5 kWh of energy in a mass of 1 kg. It weighs 2 kg in its case.";
        let json = r#"{"claims":[
            {"limit_id":null,"quantity":"energy stored","value":5,"unit":"kWh","evidence":"The Volta cell stores 5 kWh of energy in a mass of 1 kg.","constraints":{},"parameters":{}},
            {"limit_id":null,"quantity":"mass","value":1,"unit":"kg","evidence":"The Volta cell stores 5 kWh of energy in a mass of 1 kg.","constraints":{},"parameters":{}},
            {"limit_id":null,"quantity":"mass","value":2,"unit":"kg","evidence":"It weighs 2 kg in its case.","constraints":{},"parameters":{}}
        ],"missing":[]}"#;
        let out = check(&parse_model_output(json).expect("json"), text, &catalog());
        assert_eq!(
            out.len(),
            2,
            "the mass of another sentence is not merged: {out:?}"
        );
        let c = &out[0];
        assert_eq!(c.quantity, "specific energy");
        assert_eq!(c.raw_value, 5.0);
        assert_eq!(c.raw_unit, "kWh/kg");
        assert_eq!(
            limina_core::parse_unit(&c.raw_unit).map(|u| u.dimension.exponents()),
            Ok([2, 0, -2, 0, 0, 0, 0])
        );
        assert!(
            c.notes.iter().any(|n| n.contains(MERGED_QUOTIENT)),
            "{:?}",
            c.notes
        );
        assert_eq!(out[1].raw_value, 2.0);
    }

    #[test]
    fn short_numbers_hide_last_bit_noise_only() {
        assert_eq!(short_number(1.0000000000000001e-24), "1e-24");
        assert_eq!(short_number(1.0000000000000001e-7), "1e-7");
        assert_eq!(short_number(5.272859088230782e-11), "5.272859088e-11");
        assert_eq!(short_number(300.0), "300");
        assert_eq!(short_number(14.5), "14.5");
        assert_eq!(short_number(0.0), "0");
        assert_eq!(short_number(3.6e8), "3.6e8");
        assert_eq!(short_number(-0.25), "-0.25");
    }

    #[test]
    fn equivalent_unit_spellings_are_the_same_unit_but_nothing_is_converted() {
        assert!(same_unit("kg·m/s", "kg*m/s"));
        assert!(same_unit("kg m s^-1", "kg*m/s"));
        assert!(
            !same_unit("g*m/s", "kg*m/s"),
            "another factor is another unit"
        );
        assert!(!same_unit("°C", "K"));
        assert!(!same_unit("mK", "K"));
    }

    #[test]
    fn powers_of_ten_are_read_whole_and_the_mantissa_alone_is_not_a_number() {
        for (text, value) in [
            ("position uncertainty of 1 × 10⁻¹⁰ meters", 1e-10),
            ("of 1 x 10^-24 kg m/s", 1e-24),
            ("6.6 · 10^(-34) J s", 6.6e-34),
            ("2,5×10⁸ m/s", 2.5e8),
            ("3 * 10^8", 3e8),
            ("1.5 × 10¹¹ m", 1.5e11),
            ("10⁻¹⁰ m", 1e-10),
            ("10^-3 K", 1e-3),
            ("4 × 10−7 H/m", 4e-7),
        ] {
            assert!(
                !number_in("between 10-20 K", 1e-20),
                "a range is not a power of ten"
            );
            assert!(
                number_in(text, value),
                "{text} gives {value:e}: {:?}",
                numbers(text)
            );
        }
        assert!(!number_in("position uncertainty of 1 × 10⁻¹⁰ meters", 1.0));
        assert!(!number_in("position uncertainty of 1 × 10⁻¹⁰ meters", 10.0));
        assert!(!number_in("of 1 x 10^-24 kg m/s", 24.0));
        // Ordinary numbers are unchanged.
        assert_eq!(numbers("between 10 and 300 K"), vec![10.0, 300.0]);
        assert_eq!(numbers("COP of 14.5, 300 K x 2"), vec![14.5, 300.0, 2.0]);
        assert_eq!(numbers("110^"), vec![110.0]);
    }
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
            "a figure of merit of 14.5",
            r#"{"claims":[{"limit_id":null,"quantity":"figure of merit","value":14.5,"unit":"dimensionless","evidence":"figure of merit of 14.5","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(c.not_evaluable_reason(), Some(LIMIT_NOT_RECOGNIZED));
    }

    /// The text of an image wraps "308" and "K" across two lines; the model
    /// quotes "308 K" with a space and gives no evidence for Tc.
    #[test]
    fn a_wrapped_line_keeps_its_sentence_and_the_cold_reservoir_is_read_by_its_word() {
        let text = "The Boreal heat pump delivers heat with a COP of 4.0, hot reservoir at 308\nK, cold reservoir at 273 K.";
        let c = &checked(
            text,
            r#"{"claims":[{"limit_id":"cop","quantity":"coefficient of performance","value":4.0,"unit":"dimensionless","evidence":"COP of 4.0","constraints":{},"parameters":{
                "Th":{"value":308,"unit":"K","evidence":"hot reservoir at 308 K"},
                "Tc":{"value":273,"unit":"K","evidence":"cold reservoir 273 K"}}}],"missing":[]}"#,
        )[0];
        assert_eq!(
            c.parameters["Th"].value,
            Some(308.0),
            "{:?}",
            c.parameters["Th"]
        );
        assert_eq!(
            c.parameters["Th"]
                .evidence
                .as_ref()
                .map(|e| e.text.as_str()),
            Some("hot reservoir at 308\nK")
        );
        assert_eq!(
            c.parameters["Tc"].value,
            Some(273.0),
            "{:?}",
            c.parameters["Tc"]
        );
        let tc = c.parameters["Tc"].evidence.as_ref().expect("Tc evidence");
        let chars: Vec<char> = text.chars().collect();
        assert_eq!(chars[tc.start..tc.end].iter().collect::<String>(), tc.text);
        assert_eq!(tc.text, "273 K");
        assert!(c.evaluable());
    }

    #[test]
    fn two_temperatures_with_no_naming_word_are_left_alone() {
        let text = "The pump works between 308 K and 273 K with a COP of 4.0.";
        let c = &checked(
            text,
            r#"{"claims":[{"limit_id":"cop","quantity":"COP","value":4.0,"unit":"dimensionless","evidence":"COP of 4.0","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(c.parameters.get("Th").and_then(|p| p.value), None);
        assert_eq!(c.parameters.get("Tc").and_then(|p| p.value), None);
    }

    #[test]
    fn a_blank_line_still_ends_the_sentence() {
        let text = "A pump has a COP of 4.0\n\nThe cold reservoir is at 273 K.";
        let c = &checked(
            text,
            r#"{"claims":[{"limit_id":"cop","quantity":"COP","value":4.0,"unit":"dimensionless","evidence":"COP of 4.0","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(c.parameters.get("Tc").and_then(|p| p.value), None);
    }

    #[test]
    fn a_quote_is_found_across_a_line_break_only() {
        let text = "hot reservoir at 308\nK, cold";
        let s = locate(text, "308 K").expect("found");
        assert_eq!((s.start, s.end, s.text.as_str()), (17, 22, "308\nK"));
        assert!(locate(text, "308K").is_none());
        assert!(locate(text, "309 K").is_none());
    }

    #[test]
    fn a_built_in_law_is_matched_when_the_model_gives_no_limit() {
        let c = &checked(
            "a COP of 14.5",
            r#"{"claims":[{"limit_id":null,"quantity":"COP","value":14.5,"unit":"dimensionless","evidence":"COP of 14.5","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(c.limit_id.as_deref(), Some("cop"));
        assert!(
            !c.flags.iter().any(|f| f == LIMIT_NOT_RECOGNIZED),
            "{:?}",
            c.flags
        );
        assert!(
            c.notes.iter().any(|n| n.starts_with(MATCHED_BUILT_IN)),
            "{:?}",
            c.notes
        );
    }

    #[test]
    fn the_model_hint_is_overridden_by_a_keyword_and_kept_when_it_agrees() {
        // The model says Carnot for a COP: the keyword and the dimension choose cop.
        let c = &checked(
            "a COP of 14.5",
            r#"{"claims":[{"limit_id":"carnot","quantity":"coefficient of performance","value":14.5,"unit":"dimensionless","evidence":"COP of 14.5","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(c.limit_id.as_deref(), Some("cop"));
        let agreed = &checked(
            "a COP of 14.5",
            r#"{"claims":[{"limit_id":"cop","quantity":"COP","value":14.5,"unit":"dimensionless","evidence":"COP of 14.5","constraints":{},"parameters":{}}],"missing":[]}"#,
        )[0];
        assert_eq!(agreed.limit_id.as_deref(), Some("cop"));
        assert!(agreed.notes.is_empty(), "{:?}", agreed.notes);
    }

    #[test]
    fn fractions_print_with_at_least_two_decimals() {
        assert_eq!(fraction_text(0.6), "0.60");
        assert_eq!(fraction_text(0.145), "0.145");
        assert_eq!(fraction_text(1.0), "1.00");
        assert_eq!(plain(60.0), "60");
        assert_eq!(plain(14.5), "14.5");
    }

    fn shannon_catalog() -> Catalog {
        Catalog {
            limits: vec![CatalogLimit {
                id: "shannon_hartley".into(),
                title: "Shannon-Hartley limit".into(),
                formula: "B*ln(1 + snr)/ln2".into(),
                claim_unit: "bit/s".into(),
                parameters: vec![
                    crate::catalog::CatalogParameter {
                        name: "B".into(),
                        units: vec!["Hz".into(), "kHz".into(), "MHz".into(), "GHz".into()],
                    },
                    crate::catalog::CatalogParameter {
                        name: "snr".into(),
                        units: vec!["dimensionless".into()],
                    },
                ],
                assumptions: vec![],
            }],
        }
    }

    /// Seen live: the model converted "1 MHz" to 1e6 Hz, a value the text
    /// does not write. The number written in an allowed unit is read again.
    #[test]
    fn a_parameter_the_model_converted_is_read_again_as_written() {
        let text = "The Echo modem transmits 3 Mbit/s over a 1 MHz noisy channel with a linear\nsignal-to-noise ratio of 15.";
        let json = r#"{"claims":[{"limit_id":"shannon_hartley","quantity":"data rate","value":3,"unit":"bit/s","evidence":"3 Mbit/s","constraints":{},
            "parameters":{"B":{"value":1e6,"unit":"Hz","evidence":"1 MHz"},"snr":{"value":15,"unit":"dimensionless","evidence":"signal-to-noise ratio of 15"}}}],"missing":[]}"#;
        let out = check(
            &parse_model_output(json).expect("json"),
            text,
            &shannon_catalog(),
        );
        let b = &out[0].parameters["B"];
        assert_eq!((b.value, b.unit.as_str()), (Some(1.0), "MHz"), "{out:?}");
        assert_eq!(out[0].unit, "bit/s");
        assert_eq!(out[0].value, Some(3e6));
    }

    #[test]
    fn a_named_dimensionless_unit_is_never_the_same_as_one_without_it() {
        assert!(!same_unit("MHz", "Mbit/s"));
        assert!(!same_unit("rad", "dimensionless"));
        assert!(!same_unit("sr", "rad"));
        assert!(same_unit("Mbit/s", "Mbit/s"));
        assert!(same_unit("kg·m/s", "kg*m/s"));
        assert!(same_unit("kHz", "kHz"));
    }

    fn mass_energy_catalog() -> Catalog {
        Catalog {
            limits: vec![
                CatalogLimit {
                    id: "mass_energy".into(),
                    title: "Mass-energy equivalence".into(),
                    formula: "dm*c^2".into(),
                    claim_unit: "J".into(),
                    parameters: vec![crate::catalog::CatalogParameter {
                        name: "dm".into(),
                        units: vec!["kg".into(), "g".into()],
                    }],
                    assumptions: vec![],
                },
                CatalogLimit {
                    id: "two_masses".into(),
                    title: "Two masses".into(),
                    formula: "(m_initial - m_final)*c^2".into(),
                    claim_unit: "J".into(),
                    parameters: vec![
                        crate::catalog::CatalogParameter {
                            name: "m_initial".into(),
                            units: vec!["kg".into()],
                        },
                        crate::catalog::CatalogParameter {
                            name: "m_final".into(),
                            units: vec!["kg".into()],
                        },
                    ],
                    assumptions: vec![],
                },
            ],
        }
    }

    /// Seen live: one "1 kg" filled both masses of a law that needs two.
    #[test]
    fn one_written_number_never_fills_two_parameters() {
        let text = "The Nova reactor releases 2e17 J of energy by converting 1 kg of mass.";
        let json = r#"{"claims":[{"limit_id":"two_masses","quantity":"energy released","value":2e17,"unit":"J","evidence":"2e17 J","constraints":{},"parameters":{}}],"missing":[]}"#;
        let c = &check(
            &parse_model_output(json).expect("json"),
            text,
            &mass_energy_catalog(),
        )[0];
        let filled = c.parameters.values().filter(|p| p.value.is_some()).count();
        assert!(filled <= 1, "{c:?}");
        let json = json.replace("two_masses", "mass_energy");
        let c = &check(
            &parse_model_output(&json).expect("json"),
            text,
            &mass_energy_catalog(),
        )[0];
        assert_eq!(c.parameters["dm"].value, Some(1.0), "{c:?}");
    }
}
