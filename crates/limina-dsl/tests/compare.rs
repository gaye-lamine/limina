//! Two definitions of one law compared on 200 deterministic samples, and
//! the bound curve (spec two-model-laws).
use std::collections::BTreeMap;

use limina_core::{ConstraintsMap, Limit};
use limina_dsl::{
    bound_curve, compare_limits, min_compared, relative_difference, CurveError, DeclarativeLimit,
    Registry, COMPARISON_SAMPLES, COMPARISON_SEED,
};
use serde_json::json;

fn landauer(id: &str, param: &str, formula: &str, range: [f64; 2]) -> DeclarativeLimit {
    DeclarativeLimit::from_json(
        &json!({
            "id": id, "title": "Landauer", "direction": "min", "bounded_dimension": "J",
            "parameters": [{"name": param, "kind": "absolute_temperature", "units": ["K"], "test_range": range}],
            "formula": formula, "validity": [format!("{param} > 0")], "epistemic_status": "theorem"
        })
        .to_string(),
    )
    .expect("definition")
}

fn compare(a: &DeclarativeLimit, b: &DeclarativeLimit) -> limina_dsl::Comparison {
    compare_limits(a, b, COMPARISON_SAMPLES, COMPARISON_SEED)
}

#[test]
fn the_same_law_written_twice_agrees_on_200_samples() {
    let a = landauer("draft-a", "T", "kB*T*ln2", [1.0, 1000.0]);
    let b = landauer("draft-b", "T", "ln2*T*kB", [0.5, 2000.0]);
    let c = compare(&a, &b);
    assert!(c.agreed, "{c:?}");
    assert_eq!(c.samples, 200);
    assert!(c.max_relative_difference <= 1e-6);
    assert_eq!(c.reason, None);
}

#[test]
fn a_different_factor_is_disputed_with_the_point() {
    let a = landauer("draft-a", "T", "kB*T*ln2", [1.0, 1000.0]);
    let b = landauer("draft-b", "T", "2*kB*T*ln2", [1.0, 1000.0]);
    let c = compare(&a, &b);
    assert!(!c.agreed);
    assert_eq!(c.samples, 1, "stops at the first differing sample");
    let reason = c.reason.expect("reason");
    assert!(reason.contains("bounds differ at T = "), "{reason}");
    assert!((c.max_relative_difference - 0.5).abs() < 1e-12);
}

#[test]
fn a_difference_below_the_tolerance_agrees_and_above_does_not() {
    let a = landauer("draft-a", "T", "kB*T*ln2", [1.0, 1000.0]);
    let tiny = landauer("draft-b", "T", "kB*T*ln2*(1 + 1e-9)", [1.0, 1000.0]);
    assert!(compare(&a, &tiny).agreed);
    let small = landauer("draft-b", "T", "kB*T*ln2*(1 + 1e-5)", [1.0, 1000.0]);
    assert!(!compare(&a, &small).agreed);
}

#[test]
fn different_parameters_direction_or_ranges_are_disputed() {
    let a = landauer("draft-a", "T", "kB*T*ln2", [1.0, 1000.0]);
    let renamed = landauer("draft-b", "Temp", "kB*Temp*ln2", [1.0, 1000.0]);
    assert!(compare(&a, &renamed)
        .reason
        .expect("reason")
        .contains("parameters differ"));
    let apart = landauer("draft-b", "T", "kB*T*ln2", [2000.0, 3000.0]);
    assert!(compare(&a, &apart)
        .reason
        .expect("reason")
        .contains("do not overlap"));
    let registry = Registry::builtin().expect("builtin");
    let carnot = registry.get("carnot").expect("carnot");
    assert!(!compare(&a, carnot).agreed);
}

#[test]
fn the_built_in_carnot_agrees_with_itself_including_invalid_points() {
    let registry = Registry::builtin().expect("builtin");
    let carnot = registry.get("carnot").expect("carnot");
    let c = compare(carnot, carnot);
    assert!(c.agreed, "{c:?}");
    assert_eq!(c.max_relative_difference, 0.0);
}

#[test]
fn an_extra_assumption_with_the_same_bound_still_agrees() {
    let a = landauer("draft-a", "T", "kB*T*ln2", [1.0, 1000.0]);
    let b = DeclarativeLimit::from_json(
        &json!({
            "id": "draft-b", "title": "Landauer", "direction": "min", "bounded_dimension": "J",
            "parameters": [{"name": "T", "kind": "absolute_temperature", "units": ["K"], "test_range": [1, 1000]}],
            "assumptions": [{"id": "process", "options": [{"id": "quasi_static"}]}],
            "formula": "kB*T*ln2", "validity": ["T > 0"], "epistemic_status": "theorem"
        })
        .to_string(),
    )
    .expect("definition");
    assert!(compare(&a, &b).agreed);
}

#[test]
fn relative_difference_is_symmetric_and_zero_on_equality() {
    assert_eq!(relative_difference(0.0, 0.0), 0.0);
    assert_eq!(relative_difference(2.0, 1.0), relative_difference(1.0, 2.0));
    assert_eq!(relative_difference(2.0, 1.0), 0.5);
}

#[test]
fn the_carnot_curve_follows_one_minus_tc_over_th() {
    let registry = Registry::builtin().expect("builtin");
    let carnot = registry.get("carnot").expect("carnot");
    let fixed = BTreeMap::from([("Tc".to_string(), 300.0)]);
    let points = bound_curve(
        carnot,
        &ConstraintsMap::new(),
        &fixed,
        "Th",
        (200.0, 1000.0),
        5,
    )
    .expect("curve");
    assert_eq!(points.len(), 5);
    assert_eq!(points[0].x, 200.0);
    assert_eq!(points[0].b_max, None, "Th <= Tc is undefined, not a bound");
    assert_eq!(points[4].x, 1000.0);
    assert_eq!(points[4].b_max, Some(1.0 - 300.0 / 1000.0));
    assert!(carnot.parameters().len() == 2);
}

#[test]
fn curve_requests_are_checked() {
    let registry = Registry::builtin().expect("builtin");
    let carnot = registry.get("carnot").expect("carnot");
    let none = BTreeMap::new();
    let run = |fixed: &BTreeMap<String, f64>, free: &str, range, n| {
        bound_curve(carnot, &ConstraintsMap::new(), fixed, free, range, n)
    };
    assert_eq!(
        run(&none, "Th", (400.0, 900.0), 10),
        Err(CurveError::MissingFixedParameter("Tc".into()))
    );
    let fixed = BTreeMap::from([("Tc".to_string(), 300.0)]);
    assert_eq!(
        run(&fixed, "X", (400.0, 900.0), 10),
        Err(CurveError::UnknownFreeParameter("X".into()))
    );
    assert!(matches!(
        run(&fixed, "Th", (900.0, 400.0), 10),
        Err(CurveError::Invalid(_))
    ));
    assert!(matches!(
        run(&fixed, "Th", (400.0, 900.0), 1),
        Err(CurveError::Invalid(_))
    ));
    assert!(matches!(
        run(&fixed, "Th", (400.0, 900.0), 1001),
        Err(CurveError::Invalid(_))
    ));
}

/// A dimensionless law of one parameter x, for the comparison edge cases.
fn plain(id: &str, formula: &str, range: [f64; 2]) -> DeclarativeLimit {
    DeclarativeLimit::from_json(
        &json!({
            "id": id, "title": "t", "direction": "max", "bounded_dimension": "dimensionless",
            "parameters": [{"name": "x", "dimension": "dimensionless", "units": ["dimensionless"], "test_range": range}],
            "formula": formula, "validity": [], "references": ["r"], "epistemic_status": "theorem"
        })
        .to_string(),
    )
    .expect("definition")
}

/// Regression (26/11 audit): two formulas that differ by a factor 1000 but
/// are both undefined on the whole sampled range (square root of a negative)
/// used to "agree" on 200 samples without one number compared.
#[test]
fn two_formulas_undefined_everywhere_never_agree() {
    let a = plain("draft-a", "sqrt(x^2 - 100)", [1.0, 5.0]);
    let b = plain("draft-b", "1000*sqrt(x^2 - 100)", [1.0, 5.0]);
    let c = compare(&a, &b);
    assert!(!c.agreed, "{c:?}");
    assert_eq!(c.samples, 200);
    assert_eq!(c.compared, 0);
    assert!(
        c.reason
            .as_deref()
            .unwrap_or("")
            .contains("only 0 of 200 samples"),
        "{c:?}"
    );
}

/// Agreement needs a quarter of the samples compared: a law defined on less
/// than that share of its range is not agreed, one defined on more is.
#[test]
fn agreement_needs_a_quarter_of_the_samples_compared() {
    assert_eq!(min_compared(200), 50);
    assert_eq!(min_compared(1), 1);
    // Defined only for x > 4.9 of [1, 5]: about 2 % of the samples.
    let narrow = compare(
        &plain("draft-a", "sqrt(x - 4.9)", [1.0, 5.0]),
        &plain("draft-b", "sqrt(x - 4.9)", [1.0, 5.0]),
    );
    assert!(!narrow.agreed, "{narrow:?}");
    assert!(narrow.compared < 50, "{narrow:?}");
    // Defined for x > 2 of [1, 5]: about 75 % of the samples.
    let wide = compare(
        &plain("draft-a", "sqrt(x - 2)", [1.0, 5.0]),
        &plain("draft-b", "sqrt(x - 2)", [1.0, 5.0]),
    );
    assert!(wide.agreed, "{wide:?}");
    assert!(wide.compared >= 50 && wide.compared < 200, "{wide:?}");
}

/// The comparison reports where it sampled: the intersection of the two
/// ranges. Formulas that differ only outside it agree, and the range tells
/// the caller not to apply the law there.
#[test]
fn the_compared_ranges_are_the_intersection_of_the_two_definitions() {
    let a = plain("draft-a", "x", [1.0, 2.0]);
    let b = plain("draft-b", "min(x, 2) + 5*max(x - 2, 0)", [1.0, 100.0]);
    let c = compare(&a, &b);
    assert!(c.agreed, "{c:?}");
    assert_eq!(c.compared, 200);
    assert_eq!(c.ranges, vec![("x".to_string(), 1.0, 2.0)]);
}

/// A division by zero inside the range is undefined at one point at most:
/// it does not prevent agreement when the formulas match elsewhere.
#[test]
fn a_singular_point_is_not_compared_and_does_not_block_agreement() {
    let a = plain("draft-a", "1/(x - 10)", [1.0, 20.0]);
    let b = plain("draft-b", "1/(x - 10)", [1.0, 20.0]);
    let c = compare(&a, &b);
    assert!(c.agreed, "{c:?}");
    assert!(c.compared >= 199, "{c:?}");
}
