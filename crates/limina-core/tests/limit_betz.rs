//! Betz limit through the decision engine (spec limit-betz, AC 2 to 7).

mod common;

use std::collections::BTreeSet;

use common::scenario;
use limina_core::{
    decide, AssumptionId, Betz, Claim, ConstraintsMap, DecisionConfig, DecisionInput, Dimension,
    EngineError, Measure, OptionId, Quantity, Verdict,
};
use proptest::prelude::*;

const UNSHROUDED: &str = "unshrouded_rotor";
const TOTAL_AREA: &str = "shrouded_total_capture_area";
const ROTOR_ONLY: &str = "shrouded_rotor_area_only";

fn input(value: f64, options: &[&str]) -> DecisionInput {
    let mut constraints = ConstraintsMap::new();
    if !options.is_empty() {
        let set: BTreeSet<OptionId> = options.iter().map(|o| OptionId::new(*o)).collect();
        constraints.allow(AssumptionId::new("reference_area"), set);
    }
    let value = Measure::Plain(
        Quantity::from_si(value, Dimension::DIMENSIONLESS).expect("finite claim value"),
    );
    DecisionInput {
        claim: Claim {
            value,
            uncertainty: None,
        },
        config: DecisionConfig::default(),
        constraints,
        known_parameters: Default::default(),
    }
}

fn verdict(value: f64, options: &[&str]) -> Result<Verdict, EngineError> {
    decide(&Betz::new(), &input(value, options)).map(|d| d.verdict)
}

fn not_applicable(result: Result<Verdict, EngineError>) -> bool {
    matches!(result, Err(EngineError::NotApplicable { .. }))
}

#[test]
fn ac2_030_unshrouded_is_compatible() {
    assert_eq!(verdict(0.30, &[UNSHROUDED]), Ok(Verdict::Compatible));
}

#[test]
fn ac3_070_unshrouded_is_impossible() {
    assert_eq!(verdict(0.70, &[UNSHROUDED]), Ok(Verdict::Impossible));
}

#[test]
fn ac4_070_shrouded_total_capture_area_is_impossible() {
    assert_eq!(verdict(0.70, &[TOTAL_AREA]), Ok(Verdict::Impossible));
}

#[test]
fn ac5_rotor_area_only_is_not_applicable_for_any_claim() {
    for v in [0.0, 0.30, 0.70, 5.0] {
        assert!(not_applicable(verdict(v, &[ROTOR_ONLY])), "claim {v}");
    }
}

#[test]
fn ac5_rotor_area_only_mixed_with_a_valid_option_is_not_applicable() {
    assert!(not_applicable(verdict(0.70, &[UNSHROUDED, ROTOR_ONLY])));
}

#[test]
fn ac6_unconstrained_claim_is_not_applicable() {
    for v in [0.30, 0.70] {
        assert!(not_applicable(verdict(v, &[])), "claim {v}");
    }
}

#[test]
fn bound_is_16_27_not_a_rounded_decimal() {
    // 0.5926 < 16/27 = 0.59259259...: a rounded bound of 0.59 would be wrong.
    assert_eq!(verdict(0.5926, &[UNSHROUDED]), Ok(Verdict::Impossible));
    assert_eq!(verdict(0.5925, &[UNSHROUDED]), Ok(Verdict::Compatible));
    assert_eq!(verdict(16.0 / 27.0, &[UNSHROUDED]), Ok(Verdict::Compatible));
}

/// Non-empty subsets of the two valid options.
fn valid_options() -> impl Strategy<Value = Vec<&'static str>> {
    prop_oneof![
        Just(vec![UNSHROUDED]),
        Just(vec![TOTAL_AREA]),
        Just(vec![UNSHROUDED, TOTAL_AREA]),
    ]
}

/// Claims in `[0, 16/27]`, with the endpoints and their neighbours favoured.
fn claim_at_or_below_bound() -> impl Strategy<Value = f64> {
    let bound = 16.0_f64 / 27.0;
    prop_oneof![
        (0.0..=1.0f64).prop_map(move |t| t * bound),
        Just(0.0),
        Just(bound),
        Just(bound.next_down()),
    ]
}

proptest! {
    #![proptest_config(scenario::config())]

    /// AC 7. Independent oracle: with no uncertainty the claim interval is
    /// the point `v <= 16/27`, so the oracle's COMPATIBLE condition
    /// (`hi <= b_min`) holds and IMPOSSIBLE is excluded.
    #[test]
    fn claim_at_or_below_bound_is_never_impossible(
        v in claim_at_or_below_bound(),
        options in valid_options(),
    ) {
        let bound = 16.0_f64 / 27.0;
        prop_assert!(v <= bound, "generator broke its contract: v={v:?}");
        let got = verdict(v, &options);
        prop_assert_eq!(
            got.clone(),
            Ok(Verdict::Compatible),
            "claim v={:?} options={:?} gave {:?}",
            v,
            options,
            got
        );
    }
}
