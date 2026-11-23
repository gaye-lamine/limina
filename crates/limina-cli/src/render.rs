//! Text and JSON output.

use std::fmt::Write;

use crate::check::{LocatedClaim, Report};

/// A number as people read it: plain between 1e-3 and 1e6, else with an
/// exponent.
pub fn number(v: f64) -> String {
    if v == 0.0 || (1e-3..1e6).contains(&v.abs()) {
        format!("{v}")
    } else {
        format!("{v:e}")
    }
}

/// `error` for IMPOSSIBLE, `ok` for COMPATIBLE, `warning` otherwise.
fn severity(c: &LocatedClaim) -> (&'static str, &'static str) {
    match c.claim.result.as_ref().map(|r| r.verdict) {
        Some("IMPOSSIBLE") => ("error", "IMPOSSIBLE"),
        Some("COMPATIBLE") => ("ok", "COMPATIBLE"),
        Some(v) => ("warning", v),
        None => ("warning", "NEEDS_INPUT"),
    }
}

fn block(out: &mut String, c: &LocatedClaim) {
    let (sev, verdict) = severity(c);
    let claim = &c.claim;
    let limit = claim.limit_id.as_deref().unwrap_or("no known limit");
    let _ = writeln!(
        out,
        "{}:{}:{}: {sev}: {verdict} {limit}",
        c.file, c.line, c.column
    );
    let evidence = claim
        .evidence
        .as_ref()
        .map(|e| format!(" (\"{}\")", e.text))
        .unwrap_or_default();
    let value = match claim.value {
        Some(v) => format!("{} {}", number(v), claim.unit),
        None => format!(
            "{} {} as read (not sent to the engine)",
            number(claim.raw_value),
            claim.raw_unit
        ),
    };
    let bound = claim
        .result
        .as_ref()
        .and_then(|r| r.bounds.as_ref())
        .map(|b| {
            let (lo, hi) = (b.permissive.b_min, b.permissive.b_max);
            if lo == hi {
                format!(", bound {}", number(lo))
            } else {
                format!(", bound {} to {}", number(lo), number(hi))
            }
        })
        .unwrap_or_default();
    let _ = writeln!(out, "  claim {value}{evidence}{bound}");
    for note in &claim.notes {
        let _ = writeln!(out, "  note: {note}");
    }
    for flag in &claim.flags {
        let _ = writeln!(out, "  flag: {flag}");
    }
    match (&claim.result, &claim.reason) {
        (Some(r), _) => {
            let _ = writeln!(out, "  {}", r.wording);
        }
        (None, Some(reason)) => {
            let _ = writeln!(out, "  needs input: {reason}");
        }
        (None, None) => {}
    }
    if let Some(s) = &c.to_settle {
        let _ = writeln!(out, "  to settle this, state: {s}");
    }
    if let Some(s) = &c.inverse {
        let _ = writeln!(out, "  {s}");
    }
}

/// One block per claim, then a summary line.
pub fn text(report: &Report) -> String {
    let mut out = String::new();
    for c in &report.claims {
        block(&mut out, c);
        out.push('\n');
    }
    for e in &report.errors {
        let _ = writeln!(
            out,
            "{}:{}:{}: error: ANALYSIS_FAILED {}\n  {}\n  this paragraph was not checked\n",
            e.file, e.line, e.column, e.code, e.message
        );
    }
    if !report.errors.is_empty() {
        let _ = writeln!(
            out,
            "{}: {} paragraph(s) could not be analysed; the check is incomplete",
            report.file,
            report.errors.len()
        );
    }
    if report.claims.is_empty() {
        let _ = writeln!(out, "{}: no numeric claim found", report.file);
    } else {
        let _ = writeln!(
            out,
            "{}: {} claims: {} impossible, {} conditional, {} compatible, {} insufficient data, {} need input",
            report.file,
            report.claims.len(),
            report.impossible(),
            report.count("CONDITIONAL"),
            report.count("COMPATIBLE"),
            report.count("INSUFFICIENT_DATA"),
            report.needs_input(),
        );
    }
    out
}

/// The report as pretty JSON.
pub fn json(report: &Report) -> String {
    match serde_json::to_string_pretty(report) {
        Ok(s) => s + "\n",
        Err(e) => format!("{{\"error\": \"{e}\"}}\n"),
    }
}
