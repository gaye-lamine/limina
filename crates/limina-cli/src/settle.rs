//! What the author would have to state to settle a claim, and the inverse
//! line. Both read the engine result and the limit metadata; neither
//! decides a verdict.

use std::collections::BTreeSet;

use limina_api::authoring::{solve_with, SolveRequestDto};
use limina_api::dto::{AnalyzedClaimDto, ClaimDto, LimitDto, ParameterDto};
use limina_api::labels::{assumption_label, option_label, parameter_label};
use limina_api::{registry, AppState};

fn limit_of(claim: &AnalyzedClaimDto) -> Option<LimitDto> {
    let id = claim.limit_id.as_deref()?;
    registry::list().limits.into_iter().find(|l| l.id == id)
}

fn verdict(claim: &AnalyzedClaimDto) -> Option<&'static str> {
    claim.result.as_ref().map(|r| r.verdict)
}

/// Assumptions of the limit with more than one option that the claim does
/// not constrain, each with its options, by display label.
fn unstated_assumptions(limit: &LimitDto, claim: &AnalyzedClaimDto) -> Vec<String> {
    limit
        .assumptions
        .iter()
        .filter(|a| a.options.len() > 1 && !claim.constraints.contains_key(&a.id))
        .map(|a| {
            let options: Vec<String> = a
                .options
                .iter()
                .map(|o| format!("\"{}\"", option_label(o)))
                .collect();
            let options = match options.split_last() {
                Some((last, [])) => last.clone(),
                Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
                None => String::new(),
            };
            format!("{}: {options}", assumption_label(&a.id))
        })
        .collect()
}

fn parameter_items(ids: &BTreeSet<String>) -> Vec<String> {
    ids.iter()
        .map(|id| format!("{} {id}", parameter_label(id)))
        .collect()
}

/// The `to settle this, state: ...` items, joined by `; `; None when the
/// claim is settled or when nothing in the metadata names what is missing.
pub fn to_settle(claim: &AnalyzedClaimDto) -> Option<String> {
    let limit = limit_of(claim)?;
    let items = match (claim.status, verdict(claim)) {
        ("needs_input", _) if claim.code == Some("NotApplicable") => {
            unstated_assumptions(&limit, claim)
        }
        (_, Some("CONDITIONAL")) => {
            let mut items = unstated_assumptions(&limit, claim);
            let depends: BTreeSet<String> = claim
                .result
                .iter()
                .flat_map(|r| r.configurations.iter())
                .flat_map(|c| c.depends_on.iter().cloned())
                .collect();
            items.extend(parameter_items(&depends));
            items
        }
        (_, Some("INSUFFICIENT_DATA")) => {
            let missing: BTreeSet<String> = claim
                .result
                .iter()
                .flat_map(|r| r.missing_parameters.iter())
                .map(|m| m.id.clone())
                .collect();
            parameter_items(&missing)
        }
        _ => Vec::new(),
    };
    (!items.is_empty()).then(|| items.join("; "))
}

/// The solver sentence for an IMPOSSIBLE or CONDITIONAL claim: each declared
/// parameter with a value is tried as the free one, in declaration order,
/// the others fixed. None when no question has an answer.
pub fn inverse(state: &AppState, claim: &AnalyzedClaimDto) -> Option<String> {
    if !matches!(verdict(claim), Some("IMPOSSIBLE" | "CONDITIONAL")) {
        return None;
    }
    let limit = limit_of(claim)?;
    let value = claim.value?;
    let known: Vec<(String, ParameterDto)> = limit
        .parameters
        .iter()
        .filter_map(|spec| {
            let p = claim.parameters.get(&spec.id)?;
            Some((
                spec.id.clone(),
                ParameterDto {
                    value: p.value?,
                    unit: p.unit.clone(),
                },
            ))
        })
        .collect();
    known.iter().find_map(|(free, _)| {
        let request = SolveRequestDto {
            limit_id: limit.id.clone(),
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
            parameters: known
                .iter()
                .filter(|(name, _)| name != free)
                .cloned()
                .collect(),
            free_parameter: free.clone(),
            range: None,
        };
        solve_with(state, &request).ok().map(|s| s.sentence)
    })
}
