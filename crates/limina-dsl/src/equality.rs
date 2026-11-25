//! Equality laws: a law that predicts a value instead of bounding it.
//!
//! A definition with direction `equal` compiles into two ordinary
//! [`DeclarativeLimit`]s, an upper bound `F + |F| t` and a lower bound
//! `F - |F| t`, where `F` is the formula and `t` the relative `tolerance` of
//! the definition (0 when absent). Both go through the core engine
//! unchanged, with the claim uncertainty, the uncertainty of the constants
//! and the guard handled there. The two verdicts are then combined by
//! [`combine_verdicts`]. No new core verdict or direction is needed.
//!
//! Validity conditions are not limited to temperature orders here. When one
//! of them does not hold for the given parameters, the law does not apply:
//! the outcome is [`EqualityOutcome::OutsideModelDomain`], not an error and
//! not a verdict.
//!
//! An equality law takes no assumptions: its band is the predicted value.

use limina_core::{
    decide, Decision, DecisionInput, EngineError, Interval, MissingParameter, SatisfyingConfig,
    Support, Verdict,
};

use crate::analysis::check_condition;
use crate::definition::{DirectionName, LimitDefinition};
use crate::error::DslError;
use crate::expr::{parse_condition, Condition};
use crate::ivops::{holds, Values};
use crate::limit::{environment, DeclarativeLimit};

/// An equality law, compiled into its upper and lower bounds.
#[derive(Debug, Clone)]
pub struct EqualityLimit {
    definition: LimitDefinition,
    upper: DeclarativeLimit,
    lower: DeclarativeLimit,
    domain: Vec<Condition>,
}

/// The two engine decisions of an equality law and their combination.
#[derive(Debug, Clone)]
pub struct EqualityDecision {
    /// Combined verdict, see [`combine_verdicts`].
    pub verdict: Verdict,
    /// Decision against the upper bound `F + |F| t`.
    pub upper: Decision,
    /// Decision against the lower bound `F - |F| t`.
    pub lower: Decision,
}

/// Result of evaluating a claim against an equality law.
#[derive(Debug, Clone)]
pub enum EqualityOutcome {
    /// The law applies and the engine decided.
    Decided(Box<EqualityDecision>),
    /// A validity condition of the law does not hold for the given
    /// parameters: the law says nothing about this claim.
    OutsideModelDomain {
        /// The failed condition, as written in the definition.
        condition: String,
    },
}

fn invalid(def: &LimitDefinition, message: impl Into<String>) -> DslError {
    DslError::InvalidDefinition {
        limit_id: def.id.clone(),
        message: message.into(),
    }
}

impl EqualityLimit {
    /// Compiles a definition with direction `equal`.
    pub fn compile(definition: LimitDefinition) -> Result<EqualityLimit, DslError> {
        let def = &definition;
        if def.direction != DirectionName::Equal {
            return Err(invalid(def, "an equality law has direction equal"));
        }
        if !def.assumptions.is_empty() {
            return Err(invalid(def, "an equality law takes no assumptions"));
        }
        let tolerance = def.tolerance.unwrap_or(0.0);
        if !(tolerance.is_finite() && (0.0..1.0).contains(&tolerance)) {
            return Err(invalid(def, "tolerance must be in [0, 1)"));
        }
        let formula = def
            .formula
            .as_deref()
            .ok_or_else(|| invalid(def, "no formula"))?;
        let side = |direction: DirectionName, sign: char| {
            let mut d = definition.clone();
            d.direction = direction;
            d.tolerance = None;
            d.validity.clear();
            if tolerance > 0.0 {
                d.formula = Some(format!("({formula}) {sign} abs({formula}) * {tolerance}"));
            }
            DeclarativeLimit::compile(d)
        };
        let upper = side(DirectionName::Max, '+')?;
        let lower = side(DirectionName::Min, '-')?;
        let env = environment(def)?;
        let names: Vec<String> = def.parameters.iter().map(|p| p.name.clone()).collect();
        let mut domain = Vec::new();
        for text in &def.validity {
            let condition = parse_condition(text, &names)?;
            check_condition(&condition, &env)?;
            domain.push(condition);
        }
        Ok(EqualityLimit {
            definition,
            upper,
            lower,
            domain,
        })
    }

    /// The definition it was built from.
    pub fn definition(&self) -> &LimitDefinition {
        &self.definition
    }

    /// Limit id.
    pub fn id(&self) -> &str {
        &self.definition.id
    }

    /// The upper bound `F + |F| t`, as a core limit.
    pub fn upper(&self) -> &DeclarativeLimit {
        &self.upper
    }

    /// The lower bound `F - |F| t`, as a core limit.
    pub fn lower(&self) -> &DeclarativeLimit {
        &self.lower
    }

    /// Evaluates a claim. The validity conditions are checked first on the
    /// known parameters (a condition that needs a missing parameter is left
    /// to the engine, which reports the parameter as missing); a condition
    /// that does not hold gives `OutsideModelDomain`. Otherwise both bounds
    /// go through the core engine and the verdicts are combined.
    pub fn evaluate(&self, input: &DecisionInput) -> Result<EqualityOutcome, EngineError> {
        let mut values = Values::new();
        for (id, measure) in &input.known_parameters {
            let v = measure.si_value();
            values.insert(id.as_str().to_string(), Interval::new(v, v)?);
        }
        for condition in &self.domain {
            match holds(condition, &values) {
                Ok(true) => {}
                Ok(false) => {
                    return Ok(EqualityOutcome::OutsideModelDomain {
                        condition: condition.text.clone(),
                    })
                }
                Err(EngineError::UnknownParameter(_)) => {}
                Err(e) => return Err(e),
            }
        }
        let upper = decide(&self.upper, input)?;
        let lower = decide(&self.lower, input)?;
        Ok(EqualityOutcome::Decided(Box::new(EqualityDecision {
            verdict: combine_verdicts(&upper.verdict, &lower.verdict),
            upper,
            lower,
        })))
    }
}

fn union_missing(a: &[MissingParameter], b: &[MissingParameter]) -> Vec<MissingParameter> {
    let mut out = a.to_vec();
    for m in b {
        if !out.iter().any(|x| x.id == m.id) {
            out.push(m.clone());
        }
    }
    out
}

fn merge_support(a: &Support, b: &Support) -> Support {
    match (a, b) {
        (Support::Certain, Support::Certain) => Support::Certain,
        (Support::Depends(x), Support::Certain) | (Support::Certain, Support::Depends(x)) => {
            Support::Depends(x.clone())
        }
        (Support::Depends(x), Support::Depends(y)) => {
            let mut ids = x.clone();
            for id in y {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
            Support::Depends(ids)
        }
    }
}

fn both_conditional(a: &[SatisfyingConfig], b: &[SatisfyingConfig]) -> Verdict {
    let both: Vec<SatisfyingConfig> = a
        .iter()
        .filter_map(|x| {
            b.iter()
                .find(|y| y.config == x.config)
                .map(|y| SatisfyingConfig {
                    config: x.config.clone(),
                    support: merge_support(&x.support, &y.support),
                })
        })
        .collect();
    if !both.is_empty() {
        return Verdict::Conditional { satisfying: both };
    }
    // No configuration satisfies both bounds. An equality law has no
    // assumption, so this does not arise; if it did, the conservative answer
    // is the configurations that satisfy either bound, never IMPOSSIBLE.
    let mut either = a.to_vec();
    for y in b {
        if !either.iter().any(|x| x.config == y.config) {
            either.push(y.clone());
        }
    }
    Verdict::Conditional { satisfying: either }
}

/// Combines the verdicts against the upper and the lower bound.
///
/// | upper \ lower     | IMPOSSIBLE | CONDITIONAL | COMPATIBLE  | INSUFFICIENT_DATA |
/// |-------------------|------------|-------------|-------------|-------------------|
/// | IMPOSSIBLE        | IMPOSSIBLE | IMPOSSIBLE  | IMPOSSIBLE  | IMPOSSIBLE        |
/// | CONDITIONAL       | IMPOSSIBLE | CONDITIONAL | CONDITIONAL | INSUFFICIENT_DATA |
/// | COMPATIBLE        | IMPOSSIBLE | CONDITIONAL | COMPATIBLE  | INSUFFICIENT_DATA |
/// | INSUFFICIENT_DATA | IMPOSSIBLE | INSUFFICIENT_DATA | INSUFFICIENT_DATA | INSUFFICIENT_DATA |
///
/// IMPOSSIBLE when either bound is exceeded in every configuration, since
/// the claim then cannot equal the prediction. COMPATIBLE only when both
/// bounds are respected in the most restrictive configuration. The other
/// cases are the engine's own words, merged.
pub fn combine_verdicts(upper: &Verdict, lower: &Verdict) -> Verdict {
    match (upper, lower) {
        (Verdict::Impossible, _) | (_, Verdict::Impossible) => Verdict::Impossible,
        (Verdict::Compatible, Verdict::Compatible) => Verdict::Compatible,
        (Verdict::InsufficientData { missing: a }, Verdict::InsufficientData { missing: b }) => {
            Verdict::InsufficientData {
                missing: union_missing(a, b),
            }
        }
        (Verdict::InsufficientData { missing }, _) | (_, Verdict::InsufficientData { missing }) => {
            Verdict::InsufficientData {
                missing: missing.clone(),
            }
        }
        (Verdict::Conditional { satisfying: a }, Verdict::Conditional { satisfying: b }) => {
            both_conditional(a, b)
        }
        (Verdict::Conditional { satisfying }, Verdict::Compatible)
        | (Verdict::Compatible, Verdict::Conditional { satisfying }) => Verdict::Conditional {
            satisfying: satisfying.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use limina_core::{
        Claim, Configuration, ConfigurationSet, ConstraintsMap, DecisionConfig, Measure,
        MissingStatus, ParameterId, Quantity,
    };
    use proptest::prelude::*;

    use super::*;

    const H: f64 = 6.626_070_15e-34;
    const C: f64 = 299_792_458.0;

    fn definition(json: &str) -> LimitDefinition {
        serde_json::from_str(json).expect("definition")
    }

    fn photon(tolerance: &str) -> EqualityLimit {
        EqualityLimit::compile(definition(&format!(
            r#"{{"id":"photon_energy","title":"Photon energy","direction":"equal",
            "bounded_dimension":"J","formula":"h*f","epistemic_status":"theorem"{tolerance},
            "parameters":[{{"name":"f","dimension":"Hz","units":["Hz"]}}]}}"#
        )))
        .expect("compiles")
    }

    /// E = m c^2 (1/sqrt(1 - (v/c)^2) - 1), valid for v < c.
    fn kinetic() -> EqualityLimit {
        EqualityLimit::compile(definition(
            r#"{"id":"kinetic","title":"Kinetic energy","direction":"equal",
            "bounded_dimension":"J","epistemic_status":"theorem",
            "formula":"m*c^2*(1/sqrt(1 - (v/c)^2) - 1)",
            "validity":["v < c"],
            "parameters":[
              {"name":"m","dimension":"kg","units":["kg"]},
              {"name":"v","dimension":"m/s","units":["m/s"]}]}"#,
        ))
        .expect("compiles")
    }

    fn input(value: f64, params: &[(&str, f64, &str)]) -> DecisionInput {
        DecisionInput {
            claim: Claim {
                value: Measure::Plain(Quantity::from_unit(value, "J").expect("J")),
                uncertainty: None,
            },
            config: DecisionConfig::default(),
            constraints: Default::default(),
            known_parameters: params
                .iter()
                .map(|(n, v, u)| {
                    (
                        ParameterId::new(*n),
                        Measure::Plain(Quantity::from_unit(*v, u).expect("unit")),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        }
    }

    fn verdict(limit: &EqualityLimit, input: &DecisionInput) -> Verdict {
        match limit.evaluate(input).expect("evaluates") {
            EqualityOutcome::Decided(d) => d.verdict,
            EqualityOutcome::OutsideModelDomain { condition } => {
                panic!("outside the model domain: {condition}")
            }
        }
    }

    #[test]
    fn the_prediction_is_compatible_and_far_claims_are_impossible() {
        let law = photon("");
        let f = 5.0e14;
        let e = H * f;
        let at = |claim: f64| verdict(&law, &input(claim, &[("f", f, "Hz")]));
        assert_eq!(at(e), Verdict::Compatible);
        assert_eq!(at(e * 1.01), Verdict::Impossible);
        assert_eq!(at(e * 0.99), Verdict::Impossible);
    }

    #[test]
    fn the_tolerance_widens_the_band_on_both_sides() {
        let law = photon(r#","tolerance":0.05"#);
        let f = 5.0e14;
        let e = H * f;
        let at = |claim: f64| verdict(&law, &input(claim, &[("f", f, "Hz")]));
        assert_eq!(at(e * 1.04), Verdict::Compatible);
        assert_eq!(at(e * 0.96), Verdict::Compatible);
        assert_eq!(at(e * 1.06), Verdict::Impossible);
        assert_eq!(at(e * 0.94), Verdict::Impossible);
    }

    #[test]
    fn a_missing_parameter_is_insufficient_data_never_impossible() {
        let law = photon("");
        let v = verdict(&law, &input(1.0e-19, &[]));
        match v {
            Verdict::InsufficientData { missing } => {
                assert_eq!(missing.len(), 1);
                assert_eq!(missing[0].id.as_str(), "f");
                assert_eq!(missing[0].status, MissingStatus::NoDomain);
            }
            other => panic!("expected INSUFFICIENT_DATA, got {other:?}"),
        }
    }

    #[test]
    fn a_failed_validity_condition_is_outside_the_model_domain() {
        let law = kinetic();
        let params = [("m", 1.0, "kg"), ("v", 1.2 * C, "m/s")];
        match law.evaluate(&input(1.0e17, &params)).expect("evaluates") {
            EqualityOutcome::OutsideModelDomain { condition } => assert_eq!(condition, "v < c"),
            other => panic!("expected OutsideModelDomain, got {other:?}"),
        }
        // Inside the domain the same law decides.
        let v = 0.5 * C;
        let e = C * C * (1.0 / (1.0 - 0.25_f64).sqrt() - 1.0);
        assert_eq!(
            verdict(&law, &input(e, &[("m", 1.0, "kg"), ("v", v, "m/s")])),
            Verdict::Compatible
        );
    }

    #[test]
    fn a_condition_on_a_missing_parameter_is_left_to_the_engine() {
        let law = kinetic();
        let v = verdict(&law, &input(1.0, &[("m", 1.0, "kg")]));
        assert!(matches!(v, Verdict::InsufficientData { .. }));
    }

    #[test]
    fn compile_refuses_what_an_equality_law_cannot_be() {
        let base = |extra: &str| {
            format!(
                r#"{{"id":"x","title":"x","direction":"equal","bounded_dimension":"J",
                "formula":"h*f","epistemic_status":"theorem",
                "parameters":[{{"name":"f","dimension":"Hz","units":["Hz"]}}]{extra}}}"#
            )
        };
        for extra in [
            r#","tolerance":1.0"#,
            r#","tolerance":-0.1"#,
            r#","assumptions":[{"id":"a","options":[{"id":"o"}]}]"#,
        ] {
            assert!(
                EqualityLimit::compile(definition(&base(extra))).is_err(),
                "{extra}"
            );
        }
        let max = base("").replace("\"equal\"", "\"max\"");
        assert!(EqualityLimit::compile(definition(&max)).is_err());
        let declarative = base("");
        assert!(DeclarativeLimit::from_json(&declarative).is_err());
        let tolerant = base(r#","tolerance":0.1"#).replace("\"equal\"", "\"max\"");
        assert!(DeclarativeLimit::from_json(&tolerant).is_err());
    }

    fn missing(id: &str) -> MissingParameter {
        MissingParameter {
            id: ParameterId::new(id),
            status: MissingStatus::NoDomain,
        }
    }

    fn only_configuration() -> Configuration {
        let law = photon("");
        let set = ConfigurationSet::build(law.upper(), &ConstraintsMap::new(), 8).expect("set");
        let first = set.iter().next().expect("one configuration").clone();
        first
    }

    fn conditional(support: Support) -> Verdict {
        Verdict::Conditional {
            satisfying: vec![SatisfyingConfig {
                config: only_configuration(),
                support,
            }],
        }
    }

    #[test]
    fn the_combination_table_is_exhaustive() {
        let i = Verdict::Impossible;
        let k = conditional(Support::Certain);
        let c = Verdict::Compatible;
        let d = Verdict::InsufficientData {
            missing: vec![missing("f")],
        };
        let all = [&i, &k, &c, &d];
        let kind = |v: &Verdict| match v {
            Verdict::Impossible => 'I',
            Verdict::Conditional { .. } => 'K',
            Verdict::Compatible => 'C',
            Verdict::InsufficientData { .. } => 'D',
        };
        // rows: upper I K C D; columns: lower I K C D.
        let expected = ["IIII", "IKKD", "IKCD", "IDDD"];
        for (row, upper) in all.iter().enumerate() {
            for (col, lower) in all.iter().enumerate() {
                let got = kind(&combine_verdicts(upper, lower));
                let want = expected[row].chars().nth(col).expect("column");
                assert_eq!(got, want, "upper {} lower {}", kind(upper), kind(lower));
            }
        }
    }

    #[test]
    fn combined_details_are_merged() {
        let d = |ids: &[&str]| Verdict::InsufficientData {
            missing: ids.iter().map(|i| missing(i)).collect(),
        };
        match combine_verdicts(&d(&["a", "b"]), &d(&["b", "c"])) {
            Verdict::InsufficientData { missing } => {
                let ids: Vec<&str> = missing.iter().map(|m| m.id.as_str()).collect();
                assert_eq!(ids, ["a", "b", "c"]);
            }
            other => panic!("{other:?}"),
        }
        let depends = conditional(Support::Depends(vec![ParameterId::new("p")]));
        let certain = conditional(Support::Certain);
        match combine_verdicts(&certain, &depends) {
            Verdict::Conditional { satisfying } => {
                assert_eq!(satisfying.len(), 1);
                assert_eq!(
                    satisfying[0].support,
                    Support::Depends(vec![ParameterId::new("p")])
                );
            }
            other => panic!("{other:?}"),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: if std::env::var_os("CI").is_some() { 1024 } else { 256 },
            rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
            rng_seed: proptest::test_runner::RngSeed::Fixed(42),
            ..ProptestConfig::default()
        })]

        /// A claim equal to the prediction is never IMPOSSIBLE, with or
        /// without a tolerance.
        #[test]
        fn the_prediction_is_never_impossible(
            f in 1.0e9f64..1.0e17,
            tolerance in prop::sample::select(vec!["", ",\"tolerance\":0.001", ",\"tolerance\":0.2"]),
            wobble in -1.0e-12f64..1.0e-12,
        ) {
            let law = photon(tolerance);
            let claim = H * f * (1.0 + wobble);
            prop_assert_ne!(verdict(&law, &input(claim, &[("f", f, "Hz")])), Verdict::Impossible);
        }

        /// A claim outside the band by more than the tolerance is IMPOSSIBLE;
        /// inside the band it never is.
        #[test]
        fn outside_the_band_is_impossible_inside_it_is_not(
            f in 1.0e9f64..1.0e17,
            ratio in 0.0f64..3.0,
        ) {
            let tolerance = 0.1;
            let law = photon(",\"tolerance\":0.1");
            let claim = H * f * ratio;
            let v = verdict(&law, &input(claim, &[("f", f, "Hz")]));
            if (ratio - 1.0).abs() > tolerance + 1e-6 {
                prop_assert_eq!(v, Verdict::Impossible);
            } else if (ratio - 1.0).abs() < tolerance - 1e-6 {
                prop_assert_ne!(v, Verdict::Impossible);
            }
        }
    }
}
