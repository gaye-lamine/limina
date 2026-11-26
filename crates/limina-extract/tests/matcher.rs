//! The deterministic built-in law matcher: keywords plus dimension, the
//! model's limit id as a hint only, and properties that keep it safe.
use limina_extract::matcher::{laws_with_keywords, match_built_in, MatchedBy};
use limina_extract::parse::ModelClaim;
use proptest::prelude::*;
use std::collections::BTreeMap;

fn catalog() -> limina_extract::Catalog {
    limina_extract::testing::catalog()
}

fn claim(limit: Option<&str>, quantity: &str, unit: &str, evidence: &str) -> ModelClaim {
    ModelClaim {
        limit_id: limit.map(str::to_string),
        quantity: quantity.to_string(),
        value: 1.0,
        unit: unit.to_string(),
        evidence: evidence.to_string(),
        constraints: BTreeMap::new(),
        parameters: BTreeMap::new(),
    }
}

#[test]
fn every_law_with_keywords_is_a_real_catalog_id_shape() {
    for id in laws_with_keywords() {
        assert!(
            !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{id}"
        );
    }
}

#[test]
fn a_weak_keyword_needs_a_parameter_of_the_law_in_the_sentence() {
    let c = catalog();
    let with_k = "The engine reaches an efficiency of 0.6 between 600 K and 300 K.";
    let m = match_built_in(
        &claim(None, "efficiency", "dimensionless", with_k),
        with_k,
        &c,
    );
    assert_eq!(m.map(|m| m.id), Some("carnot".to_string()));
    let without = "The solar panel reaches an efficiency of 0.6.";
    assert_eq!(
        match_built_in(
            &claim(None, "efficiency", "dimensionless", without),
            without,
            &c
        ),
        None
    );
}

#[test]
fn the_hint_breaks_nothing_it_is_kept_only_with_the_right_dimension() {
    let c = catalog();
    let t = "a figure of merit of 0.5";
    let kept = match_built_in(
        &claim(Some("betz"), "figure of merit", "dimensionless", t),
        t,
        &c,
    );
    assert_eq!(
        kept.map(|m| (m.id, m.by)),
        Some(("betz".to_string(), MatchedBy::Hint))
    );
    let t2 = "a length of 5 m";
    assert_eq!(
        match_built_in(&claim(Some("betz"), "length", "m", t2), t2, &c),
        None
    );
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: proptest::test_runner::RngSeed::Fixed(20261126), ..ProptestConfig::default() })]

    /// Whatever the quantity name and the hint, a match always has the
    /// dimension of the claim unit: a law of another dimension is never chosen.
    #[test]
    fn a_match_always_has_the_claim_dimension(
        quantity in "[a-z ]{0,30}",
        hint in prop::sample::select(vec![None, Some("betz"), Some("carnot"), Some("cop"), Some("nope")]),
        unit in prop::sample::select(vec!["m", "K", "J", "m/s", "dimensionless", "%", "kg"]),
    ) {
        let c = catalog();
        let text = format!("{quantity} 1 {unit}");
        if let Some(m) = match_built_in(&claim(hint, &quantity, unit, &text), &text, &c) {
            let law = c.limit(&m.id).expect("catalog law");
            let dimensionless = |u: &str| u == "dimensionless" || u == "%" || u.is_empty();
            prop_assert!(dimensionless(&law.claim_unit) == dimensionless(unit),
                "quantity {:?} unit {:?} hint {:?} matched {}", quantity, unit, hint, m.id);
        }
    }
}
