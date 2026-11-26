//! A served law with one more assumption that the text must state for the
//! law's bound to apply.
//!
//! The extra assumption has two options: one under which the wrapped law's
//! bound applies, and one under which this law gives no bound (a huge
//! stand-in for a maximum, 0 for a minimum). When the text does not state
//! the first option, the engine evaluates both: a claim beyond the bound is
//! then CONDITIONAL (possible only under the open option), never IMPOSSIBLE.
//! The verdict is still computed by `limina_core::decide`; nothing here
//! changes a verdict after the engine.
//!
//! Two laws use it:
//! - Carnot, with `conversion_process`: its bound holds for a heat engine
//!   cycle between two reservoirs, not for a fuel cell or a battery that
//!   happens to run at two temperatures;
//! - every law written by language models (`draft-*`), with `draft_formula`:
//!   an unvalidated formula can show that a claim is inconsistent with it,
//!   never that the claim is impossible.

use limina_core::{
    Assumption, AssumptionId, AssumptionOption, Carnot, Configuration, Direction, EngineError,
    Interval, Limit, MeasureKind, OptionId, ParameterEnclosure, ParameterSpec,
};
use limina_dsl::DeclarativeLimit;
use limina_extract::matcher::{CONVERSION_PROCESS, HEAT_ENGINE_CYCLE, NOT_A_HEAT_ENGINE_CYCLE};

use crate::dto::EvaluateResponse;

/// The bound of a maximum under the open option: far above any physical
/// value in SI units, yet small enough that the engine's guard
/// `b + 1e-9 b` stays finite.
pub const NO_BOUND_MAX: f64 = 1e300;

/// Assumption added to every law written by language models.
pub const DRAFT_FORMULA: &str = "draft_formula";
/// Option of [`DRAFT_FORMULA`]: the AI-written formula is taken as written.
pub const AS_WRITTEN: &str = "as_written";
/// Option of [`DRAFT_FORMULA`]: the formula has not been validated by a
/// person, so it bounds nothing.
pub const NOT_VALIDATED: &str = "not_validated";

/// A law with one more assumption whose open option gives no bound.
#[derive(Debug, Clone)]
pub struct Qualified<L> {
    inner: L,
    assumptions: Vec<Assumption>,
    key: AssumptionId,
    open: OptionId,
}

impl<L: Limit> Qualified<L> {
    /// `inner` with the assumption `key`: its bound under `applies`, no
    /// bound under `open`.
    pub fn new(inner: L, key: &str, applies: &str, open: &str) -> Qualified<L> {
        let mut assumptions = inner.assumptions().to_vec();
        assumptions.push(Assumption {
            id: AssumptionId::new(key),
            domain: vec![
                AssumptionOption {
                    id: OptionId::new(applies),
                    value: None,
                },
                AssumptionOption {
                    id: OptionId::new(open),
                    value: None,
                },
            ],
        });
        Qualified {
            inner,
            assumptions,
            key: AssumptionId::new(key),
            open: OptionId::new(open),
        }
    }

    /// The wrapped law.
    pub fn inner(&self) -> &L {
        &self.inner
    }

    /// Id of the added assumption.
    pub fn key(&self) -> &str {
        self.key.as_str()
    }

    /// Id of the option under which this law gives no bound.
    pub fn open(&self) -> &str {
        self.open.as_str()
    }
}

impl<L: Limit> Limit for Qualified<L> {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn direction(&self) -> Direction {
        self.inner.direction()
    }

    fn bounded_kind(&self) -> MeasureKind {
        self.inner.bounded_kind()
    }

    fn assumptions(&self) -> &[Assumption] {
        &self.assumptions
    }

    fn parameters(&self) -> &[ParameterSpec] {
        self.inner.parameters()
    }

    fn bound(
        &self,
        config: &Configuration,
        params: &ParameterEnclosure,
    ) -> Result<Interval, EngineError> {
        let option = config
            .option(&self.key)
            .ok_or_else(|| EngineError::UnknownAssumption(self.key.as_str().to_string()))?;
        if *option != self.open {
            return self.inner.bound(config, params);
        }
        Interval::point(match self.inner.direction() {
            Direction::Max => NO_BOUND_MAX,
            Direction::Min => 0.0,
        })
    }

    fn guard_abs(&self) -> f64 {
        self.inner.guard_abs()
    }
}

/// The Carnot law as served: the core limit, applied only to a heat engine
/// cycle between two reservoirs.
pub fn carnot() -> Qualified<Carnot> {
    Qualified::new(
        Carnot::new(),
        CONVERSION_PROCESS,
        HEAT_ENGINE_CYCLE,
        NOT_A_HEAT_ENGINE_CYCLE,
    )
}

/// A law written by language models, as it is evaluated: never IMPOSSIBLE
/// on its own authority, since no person has validated it.
pub fn draft(limit: &DeclarativeLimit) -> Qualified<DeclarativeLimit> {
    Qualified::new(limit.clone(), DRAFT_FORMULA, AS_WRITTEN, NOT_VALIDATED)
}

/// Whether the engine's CONDITIONAL comes only from the open option: every
/// member of C* takes it. The claim is then beyond the law's bound under
/// every other assumption.
pub fn only_open_option(response: &EvaluateResponse, key: &str, open: &str) -> bool {
    response.verdict == "CONDITIONAL"
        && !response.configurations.is_empty()
        && response
            .configurations
            .iter()
            .all(|c| c.assumptions.get(key).map(String::as_str) == Some(open))
}

/// Wording of a Carnot claim beyond the bound while the text states no heat
/// engine cycle.
pub fn carnot_wording(id: &str) -> String {
    format!(
        "inconsistent with limit {id} under the assumption of a heat engine cycle between two \
         reservoirs, which the text does not state; possible only if the conversion is not a \
         heat engine cycle"
    )
}

/// Wording of a claim beyond the bound of a law written by language models.
pub fn draft_wording(id: &str) -> String {
    format!(
        "inconsistent with draft limit {id} if its formula is correct; the formula is an \
         unvalidated draft written by AI, so no IMPOSSIBLE verdict is given"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use limina_core::{decide, Claim, DecisionConfig, DecisionInput, Measure, ParameterId};
    use limina_core::{AbsoluteTemperature, ConstraintsMap, Verdict};
    use std::collections::BTreeMap;

    fn carnot_input(
        efficiency: f64,
        th: f64,
        tc: f64,
        constraints: ConstraintsMap,
    ) -> DecisionInput {
        let t = |k: f64| {
            Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(k).expect("kelvin"))
        };
        DecisionInput {
            claim: Claim {
                value: Measure::Plain(
                    limina_core::Quantity::from_unit(efficiency, "dimensionless").expect("unit"),
                ),
                uncertainty: None,
            },
            config: DecisionConfig::default(),
            constraints,
            known_parameters: BTreeMap::from([
                (ParameterId::new("Th"), t(th)),
                (ParameterId::new("Tc"), t(tc)),
            ]),
        }
    }

    #[test]
    fn carnot_without_a_stated_heat_engine_is_conditional_and_with_one_impossible() {
        let law = carnot();
        let open = decide(
            &law,
            &carnot_input(0.65, 353.0, 293.0, ConstraintsMap::new()),
        )
        .expect("decides");
        assert!(
            matches!(open.verdict, Verdict::Conditional { .. }),
            "{:?}",
            open.verdict
        );
        let mut stated = ConstraintsMap::new();
        stated.allow(
            AssumptionId::new(CONVERSION_PROCESS),
            [OptionId::new(HEAT_ENGINE_CYCLE)].into_iter().collect(),
        );
        let heat_engine = decide(&law, &carnot_input(0.65, 353.0, 293.0, stated)).expect("decides");
        assert_eq!(heat_engine.verdict, Verdict::Impossible);
    }

    #[test]
    fn a_claim_within_the_carnot_bound_stays_compatible() {
        let r = decide(
            &carnot(),
            &carnot_input(0.10, 353.0, 293.0, ConstraintsMap::new()),
        )
        .expect("decides");
        assert_eq!(r.verdict, Verdict::Compatible);
    }
}
