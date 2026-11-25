//! Mapping of an engine `Decision` to the JSON response and its wording.

use std::collections::{BTreeMap, BTreeSet};

use limina_core::{
    ConfigurationSet, Decision, DecisionInput, EnclosureResult, EngineError, Limit,
    MissingParameter, MissingStatus, SatisfyingConfig, Support, Verdict,
};

use limina_dsl::{EqualityDecision, EqualityLimit};

use crate::dto::{
    BoundsDto, ConfigurationDto, EnclosureDto, EvaluateResponse, MarginDto, MissingParameterDto,
};
use crate::labels::{assumption_label, option_label};

/// Version of the engine build, taken from this crate's version because
/// `limina-core` exposes no version constant.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Builds the response for `decision`, computed by `decide` for `input`.
///
/// Errors: the configuration set cannot be rebuilt (it was already built
/// successfully by `decide`, so this does not happen in practice).
pub fn build(
    limit: &dyn Limit,
    input: &DecisionInput,
    decision: &Decision,
) -> Result<EvaluateResponse, EngineError> {
    let mut configurations = Vec::new();
    let mut missing_parameters = Vec::new();
    let (verdict, wording) = match &decision.verdict {
        Verdict::Impossible => {
            let set = ConfigurationSet::build(
                limit,
                &input.constraints,
                input.config.max_configurations(),
            )?;
            (
                "IMPOSSIBLE",
                format!(
                    "inconsistent with limit {} under assumptions {}",
                    limit.id(),
                    describe_assumptions(&set)
                ),
            )
        }
        Verdict::Conditional { satisfying } => {
            configurations = satisfying.iter().map(configuration_dto).collect();
            ("CONDITIONAL", conditional_wording(satisfying))
        }
        Verdict::Compatible => (
            "COMPATIBLE",
            "does not exceed the bound; this does not prove the performance is reached".to_string(),
        ),
        Verdict::InsufficientData { missing } => {
            missing_parameters = missing.iter().map(missing_dto).collect();
            ("INSUFFICIENT_DATA", insufficient_wording(missing))
        }
    };
    Ok(EvaluateResponse {
        verdict,
        limit_id: limit.id().to_string(),
        engine_version: ENGINE_VERSION,
        wording,
        bounds: decision.enclosures.map(|e| BoundsDto {
            permissive: enclosure_dto(&e.permissive),
            restrictive: enclosure_dto(&e.restrictive),
        }),
        configurations,
        missing_parameters,
    })
}

/// [`build`] for a limit authored from a text. The IMPOSSIBLE sentence says
/// "inconsistent with draft limit X under assumptions Y" ("none" when the
/// limit has no assumption); every other sentence starts with "draft limit X: ".
pub fn build_draft(
    limit: &dyn Limit,
    input: &DecisionInput,
    decision: &Decision,
) -> Result<EvaluateResponse, EngineError> {
    let mut response = build(limit, input, decision)?;
    let id = limit.id();
    response.wording = if response.verdict == "IMPOSSIBLE" {
        let plain = format!("inconsistent with limit {id} under assumptions ");
        let rest = response.wording.strip_prefix(&plain).unwrap_or("");
        let rest = if rest.is_empty() { "none" } else { rest };
        format!("inconsistent with draft limit {id} under assumptions {rest}")
    } else {
        format!("draft limit {id}: {}", response.wording)
    };
    Ok(response)
}

/// [`build`] for an equality law. The decision is the one against the upper
/// bound with the combined verdict; the bounds shown are the lower end of
/// the lower bound and the upper end of the upper bound, that is the band
/// the claim is compared with. The wording says "inconsistent with limit X
/// under assumptions none" for IMPOSSIBLE (an equality law has no assumption).
pub fn build_equality(
    law: &EqualityLimit,
    input: &DecisionInput,
    decision: &EqualityDecision,
) -> Result<EvaluateResponse, EngineError> {
    let combined = Decision {
        verdict: decision.verdict.clone(),
        ..decision.upper.clone()
    };
    let mut response = build(law.upper(), input, &combined)?;
    let id = law.id();
    response.wording = match &decision.verdict {
        Verdict::Impossible => format!("inconsistent with limit {id} under assumptions none"),
        Verdict::Compatible => format!(
            "consistent with the value predicted by limit {id}, within its tolerance band; this does not prove the performance is reached"
        ),
        _ => response.wording,
    };
    response.bounds = match (&decision.upper.enclosures, &decision.lower.enclosures) {
        (Some(up), Some(low)) => {
            let band = |u: &EnclosureResult, l: &EnclosureResult| EnclosureDto {
                b_min: l.bounds.b_min,
                b_max: u.bounds.b_max,
                margin: None,
            };
            Some(BoundsDto {
                permissive: band(&up.permissive, &low.permissive),
                restrictive: band(&up.restrictive, &low.restrictive),
            })
        }
        _ => None,
    };
    Ok(response)
}

/// `a = o` for one option, `a in {o1, o2}` for several, joined by `; `.
fn describe_assumptions(set: &ConfigurationSet) -> String {
    let mut allowed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for config in set.iter() {
        for (assumption, option) in config.iter() {
            allowed
                .entry(assumption.as_str().to_string())
                .or_default()
                .insert(option.as_str().to_string());
        }
    }
    allowed
        .iter()
        .map(|(assumption, options)| {
            let assumption = assumption_label(assumption);
            let options: Vec<&str> = options.iter().map(|o| option_label(o)).collect();
            match options.as_slice() {
                [one] => format!("{assumption} = {one}"),
                many => format!("{assumption} in {{{}}}", many.join(", ")),
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn config_text(member: &SatisfyingConfig) -> String {
    member
        .config
        .iter()
        .map(|(a, o)| {
            format!(
                "{} = {}",
                assumption_label(a.as_str()),
                option_label(o.as_str())
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn conditional_wording(satisfying: &[SatisfyingConfig]) -> String {
    let configs: Vec<String> = satisfying.iter().map(config_text).collect();
    format!("possible only if {}", configs.join(" or "))
}

fn insufficient_wording(missing: &[MissingParameter]) -> String {
    let ids: Vec<&str> = missing.iter().map(|m| m.id.as_str()).collect();
    format!("to decide, the following is needed: {}", ids.join(", "))
}

fn configuration_dto(member: &SatisfyingConfig) -> ConfigurationDto {
    let (support, depends_on) = match &member.support {
        Support::Certain => ("Certain", Vec::new()),
        Support::Depends(ids) => (
            "Depends",
            ids.iter().map(|id| id.as_str().to_string()).collect(),
        ),
    };
    ConfigurationDto {
        assumptions: member
            .config
            .iter()
            .map(|(a, o)| (a.as_str().to_string(), o.as_str().to_string()))
            .collect(),
        support,
        depends_on,
    }
}

fn missing_dto(missing: &MissingParameter) -> MissingParameterDto {
    MissingParameterDto {
        id: missing.id.as_str().to_string(),
        status: match missing.status {
            MissingStatus::NoDomain => "NoDomain",
            MissingStatus::HasDomain => "HasDomain",
        },
    }
}

fn enclosure_dto(result: &EnclosureResult) -> EnclosureDto {
    EnclosureDto {
        b_min: result.bounds.b_min,
        b_max: result.bounds.b_max,
        margin: result.margin.map(|m| MarginDto {
            low: m.low,
            high: m.high,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use limina_core::{
        decide, Assumption, AssumptionId, AssumptionOption, Claim, Configuration, ConstraintsMap,
        DecisionConfig, Dimension, Direction, DomainRange, Interval, Measure, MeasureKind,
        OptionId, ParameterEnclosure, ParameterId, ParameterSpec, PlausibleDomain, Quantity,
    };

    use super::*;

    const DIMLESS: MeasureKind = MeasureKind::Plain(Dimension::DIMENSIONLESS);

    /// Fake limit for these tests only: options `a` and `b` with bases 0.5
    /// and 0.9, multiplied by parameter `p` when it is declared.
    struct FakeLimit {
        assumptions: Vec<Assumption>,
        parameters: Vec<ParameterSpec>,
    }

    impl FakeLimit {
        fn new(with_p: bool) -> FakeLimit {
            let option = |id: &str| AssumptionOption {
                id: OptionId::new(id),
                value: None,
            };
            let parameters = if with_p {
                vec![ParameterSpec {
                    id: ParameterId::new("p"),
                    kind: DIMLESS,
                    required: true,
                    plausible: Some(PlausibleDomain {
                        range: DomainRange::Interval(Interval::new(1.0, 2.0).expect("interval")),
                        source: "test".to_string(),
                    }),
                }]
            } else {
                Vec::new()
            };
            FakeLimit {
                assumptions: vec![Assumption {
                    id: AssumptionId::new("mode"),
                    domain: vec![option("a"), option("b")],
                }],
                parameters,
            }
        }
    }

    impl Limit for FakeLimit {
        fn id(&self) -> &str {
            "fake"
        }
        fn direction(&self) -> Direction {
            Direction::Max
        }
        fn bounded_kind(&self) -> MeasureKind {
            DIMLESS
        }
        fn assumptions(&self) -> &[Assumption] {
            &self.assumptions
        }
        fn parameters(&self) -> &[ParameterSpec] {
            &self.parameters
        }
        fn bound(
            &self,
            config: &Configuration,
            params: &ParameterEnclosure,
        ) -> Result<Interval, EngineError> {
            let base = match config
                .option(&AssumptionId::new("mode"))
                .map(OptionId::as_str)
            {
                Some("a") => Interval::point(0.5)?,
                Some("b") => Interval::point(0.9)?,
                _ => return Err(EngineError::UnknownAssumption("mode".to_string())),
            };
            match params.get(&ParameterId::new("p")) {
                Some(p) => base.mul(p),
                None => Ok(base),
            }
        }
    }

    fn evaluate(limit: &FakeLimit, claim: f64, u: Option<f64>) -> EvaluateResponse {
        let input = DecisionInput {
            claim: Claim {
                value: Measure::Plain(
                    Quantity::from_unit(claim, "dimensionless").expect("valid claim"),
                ),
                uncertainty: u.map(|u| {
                    Measure::Plain(Quantity::from_unit(u, "dimensionless").expect("valid u"))
                }),
            },
            config: DecisionConfig::default(),
            constraints: ConstraintsMap::new(),
            known_parameters: BTreeMap::new(),
        };
        let decision = decide(limit, &input).expect("decision");
        build(limit, &input, &decision).expect("response")
    }

    #[test]
    fn conditional_lists_the_certain_configurations() {
        let response = evaluate(&FakeLimit::new(false), 0.7, None);
        assert_eq!(response.verdict, "CONDITIONAL");
        assert_eq!(response.wording, "possible only if mode = b");
        assert_eq!(response.configurations.len(), 1);
        assert_eq!(response.configurations[0].support, "Certain");
        assert_eq!(response.configurations[0].assumptions["mode"], "b");
        assert!(response.configurations[0].depends_on.is_empty());
    }

    #[test]
    fn impossible_lists_every_remaining_option() {
        let response = evaluate(&FakeLimit::new(false), 0.95, None);
        assert_eq!(response.verdict, "IMPOSSIBLE");
        assert_eq!(
            response.wording,
            "inconsistent with limit fake under assumptions mode in {a, b}"
        );
    }

    #[test]
    fn missing_parameter_with_a_domain_is_tagged_has_domain() {
        // p in [1, 2]: permissive gives COMPATIBLE, restrictive CONDITIONAL.
        let response = evaluate(&FakeLimit::new(true), 0.7, None);
        assert_eq!(response.verdict, "INSUFFICIENT_DATA");
        assert_eq!(response.missing_parameters.len(), 1);
        assert_eq!(response.missing_parameters[0].id, "p");
        assert_eq!(response.missing_parameters[0].status, "HasDomain");
        assert_eq!(response.wording, "to decide, the following is needed: p");
        assert!(response.bounds.is_some());
    }

    #[test]
    fn depends_tag_lists_the_missing_parameters() {
        // p in [1, 2], claim [0.85, 1.05]: option a is in [0.5, 1.0], option b in
        // [0.9, 1.8]. Both enclosures say CONDITIONAL. Only b admits the claim
        // under the restrictive enclosure, so b is Certain and a depends on p.
        let response = evaluate(&FakeLimit::new(true), 0.95, Some(0.05));
        assert_eq!(response.verdict, "CONDITIONAL");
        let tags: Vec<(&str, &str, &[String])> = response
            .configurations
            .iter()
            .map(|c| {
                (
                    c.assumptions["mode"].as_str(),
                    c.support,
                    c.depends_on.as_slice(),
                )
            })
            .collect();
        assert_eq!(
            tags,
            [
                ("a", "Depends", &["p".to_string()][..]),
                ("b", "Certain", &[][..])
            ]
        );
        assert_eq!(response.wording, "possible only if mode = a or mode = b");
    }
}
