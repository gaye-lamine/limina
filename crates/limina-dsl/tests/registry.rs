//! Registry, check report, status and interval conservativeness (spec
//! limit-dsl, AC 4 to 6).

use std::collections::{BTreeMap, BTreeSet};

use limina_core::{
    AssumptionId, Carnot, ConfigurationSet, ConstraintsMap, Cop, Interval, Limit, OptionId,
    ParameterEnclosure, ParameterId,
};
use limina_dsl::report::FormulaReport;
use limina_dsl::{
    default_dir, DeclarativeLimit, DimensionCheck, DslError, Monotonicity, Registry, Status,
};
use serde_json::json;

fn registry() -> Registry {
    Registry::load_dir(&default_dir()).expect("data/limits loads")
}

#[test]
fn the_registry_loads_betz_carnot_and_cop_as_drafts() {
    let r = registry();
    let ids: Vec<&str> = r.limits().iter().map(|l| l.id()).collect();
    assert_eq!(ids, ["betz", "carnot", "cop"]);
    for l in r.limits() {
        assert_eq!(l.definition().status, Status::Draft);
        assert!(!l.is_validated());
        assert_eq!(
            l.definition().references,
            vec!["TODO(reference): to be supplied by the author"]
        );
    }
    assert_eq!(r.as_limits().len(), 3);
}

fn trend(report: &FormulaReport, parameter: &str) -> Monotonicity {
    report
        .trends
        .iter()
        .find(|t| t.parameter == parameter)
        .map(|t| t.monotonicity)
        .expect("trend")
}

#[test]
fn carnot_is_increasing_in_th_and_decreasing_in_tc() {
    let reports = registry().reports();
    let carnot = reports
        .iter()
        .find(|r| r.limit_id == "carnot")
        .expect("carnot");
    assert_eq!(carnot.dimension, DimensionCheck::Passed);
    assert!(!carnot.validated);
    let f = &carnot.formulas[0];
    assert_eq!(f.formula.as_deref(), Some("1 - Tc/Th"));
    assert_eq!(trend(f, "Th"), Monotonicity::Increasing);
    assert_eq!(trend(f, "Tc"), Monotonicity::Decreasing);
    assert_eq!(f.finite, Some(true));
    assert_eq!(f.non_negative, Some(true));
    assert_eq!(f.samples, 81);
    for c in &carnot.validity {
        assert_eq!((c.held, c.failed), (81, 0), "{}", c.condition);
    }
}

#[test]
fn cop_bounds_decrease_in_th_and_increase_in_tc() {
    let reports = registry().reports();
    let cop = reports.iter().find(|r| r.limit_id == "cop").expect("cop");
    assert_eq!(cop.formulas.len(), 2);
    for f in &cop.formulas {
        assert_eq!(
            trend(f, "Th"),
            Monotonicity::Decreasing,
            "{}",
            f.configuration
        );
        assert_eq!(
            trend(f, "Tc"),
            Monotonicity::Increasing,
            "{}",
            f.configuration
        );
        assert_eq!(f.non_negative, Some(true));
    }
}

#[test]
fn betz_reports_the_option_without_a_formula_as_not_applicable() {
    let reports = registry().reports();
    let betz = reports.iter().find(|r| r.limit_id == "betz").expect("betz");
    let by: BTreeMap<&str, Option<&str>> = betz
        .formulas
        .iter()
        .map(|f| (f.configuration.as_str(), f.formula.as_deref()))
        .collect();
    assert_eq!(
        by.get("reference_area = shrouded_rotor_area_only"),
        Some(&None)
    );
    assert_eq!(
        by.get("reference_area = unshrouded_rotor"),
        Some(&Some("16/27"))
    );
    let f = betz
        .formulas
        .iter()
        .find(|f| f.formula.is_some())
        .expect("a formula");
    assert_eq!(
        (f.finite, f.non_negative, f.samples),
        (Some(true), Some(true), 1)
    );
}

fn definition(status: &str, validated_by: Option<&str>) -> String {
    let mut d = json!({
        "id": "x", "title": "x", "direction": "max", "bounded_dimension": "dimensionless",
        "formula": "16/27", "status": status, "epistemic_status": "model_bound"
    });
    if let Some(v) = validated_by {
        d["validated_by"] = json!(v);
    }
    d.to_string()
}

#[test]
fn a_draft_is_never_reported_as_validated() {
    let draft =
        DeclarativeLimit::from_json(&definition("draft", Some("A. Reviewer"))).expect("draft");
    assert!(!draft.is_validated());
    assert!(!limina_dsl::check_limit(&draft).validated);
    let default = DeclarativeLimit::from_json(&definition("draft", None)).expect("draft");
    assert!(!limina_dsl::check_limit(&default).validated);
    let mut no_status: serde_json::Value =
        serde_json::from_str(&definition("draft", None)).expect("json");
    no_status.as_object_mut().expect("object").remove("status");
    let implicit = DeclarativeLimit::from_json(&no_status.to_string()).expect("default status");
    assert_eq!(implicit.definition().status, Status::Draft);
    assert!(!implicit.is_validated());
}

#[test]
fn validated_needs_a_person() {
    assert!(matches!(
        DeclarativeLimit::from_json(&definition("validated", None)),
        Err(DslError::InvalidDefinition { .. })
    ));
    assert!(matches!(
        DeclarativeLimit::from_json(&definition("validated", Some("  "))),
        Err(DslError::InvalidDefinition { .. })
    ));
    let ok = DeclarativeLimit::from_json(&definition("validated", Some("A. Reviewer")))
        .expect("validated");
    assert!(ok.is_validated());
}

#[test]
fn a_repeated_id_or_a_broken_file_refuses_the_directory() {
    let d: limina_dsl::LimitDefinition =
        serde_json::from_str(&definition("draft", None)).expect("def");
    assert!(matches!(
        Registry::from_definitions(vec![d.clone(), d]),
        Err(DslError::InvalidDefinition { .. })
    ));
    assert!(matches!(
        Registry::load_dir(std::path::Path::new("no/such/dir")),
        Err(DslError::Io { .. })
    ));
}

fn config_for(limit: &dyn Limit, assumption: &str, option: &str) -> limina_core::Configuration {
    let mut constraints = ConstraintsMap::new();
    constraints.allow(
        AssumptionId::new(assumption),
        BTreeSet::from([OptionId::new(option)]),
    );
    ConfigurationSet::build(limit, &constraints, 8)
        .expect("configuration set")
        .iter()
        .next()
        .cloned()
        .expect("one configuration")
}

fn enclosure(th: (f64, f64), tc: (f64, f64)) -> ParameterEnclosure {
    let iv = |(a, b): (f64, f64)| Interval::new(a, b).expect("interval");
    ParameterEnclosure::new(BTreeMap::from([
        (ParameterId::new("Th"), iv(th)),
        (ParameterId::new("Tc"), iv(tc)),
    ]))
}

#[test]
fn interval_evaluation_with_a_repeated_variable_is_conservative() {
    let r = registry();
    let dsl = r.get("cop").expect("cop");
    let rust = Cop::new();
    let params = enclosure((300.0, 320.0), (250.0, 280.0));
    for mode in ["heating", "cooling"] {
        let exact = rust
            .bound(&config_for(&rust, "mode", mode), &params)
            .expect("rust");
        let naive = dsl
            .bound(&config_for(dsl, "mode", mode), &params)
            .expect("dsl");
        // Th appears twice in Th/(Th - Tc): the DSL range contains the exact one.
        assert!(
            naive.lo() <= exact.lo() && naive.hi() >= exact.hi(),
            "{mode}: {naive:?} vs {exact:?}"
        );
        if mode == "heating" {
            assert!(
                naive.lo() < exact.lo(),
                "strictly wider: {naive:?} vs {exact:?}"
            );
        }
    }
    // Carnot uses each temperature once: the ranges are identical.
    let carnot = r.get("carnot").expect("carnot");
    let rust = Carnot::new();
    let a = rust.bound(
        &config_for(&rust, "cycle_model", "ideal_reversible_two_reservoirs"),
        &params,
    );
    let b = carnot.bound(
        &config_for(carnot, "cycle_model", "ideal_reversible_two_reservoirs"),
        &params,
    );
    assert_eq!(a, b);
}

#[test]
fn a_wider_bound_range_never_creates_a_false_impossible() {
    use limina_core::{
        decide, AbsoluteTemperature, Claim, DecisionConfig, DecisionInput, Dimension, Measure,
        Quantity, Verdict,
    };
    // With Th = 310 K and Tc = 290 K the heating bound is 15.5: a claim at
    // or below it is never IMPOSSIBLE, and both limits agree.
    let r = registry();
    let dsl = r.get("cop").expect("cop");
    let k = |v: f64| Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(v).expect("k"));
    for claim in [10.0, 15.0, 15.5, 31.0] {
        let input = DecisionInput {
            claim: Claim {
                value: Measure::Plain(
                    Quantity::from_si(claim, Dimension::DIMENSIONLESS).expect("q"),
                ),
                uncertainty: None,
            },
            config: DecisionConfig::default(),
            constraints: ConstraintsMap::new(),
            known_parameters: BTreeMap::from([
                (ParameterId::new("Th"), k(310.0)),
                (ParameterId::new("Tc"), k(290.0)),
            ]),
        };
        let a = decide(&Cop::new(), &input).map(|d| d.verdict);
        let b = decide(dsl, &input).map(|d| d.verdict);
        assert_eq!(a, b, "claim {claim}");
        if claim <= 15.5 {
            assert_ne!(b, Ok(Verdict::Impossible));
        }
    }
}
