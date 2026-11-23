//! Dimension checker, parser errors and unknown symbols, through whole
//! definitions (spec limit-dsl, AC 2 and 3).

use limina_dsl::{DeclarativeLimit, DslError};
use serde_json::json;

/// A one-formula definition over absolute temperatures `Th`, `Tc` and `T`.
fn compile(formula: &str, bounded: serde_json::Value) -> Result<DeclarativeLimit, DslError> {
    compile_with(formula, bounded, json!([]))
}

fn compile_with(
    formula: &str,
    bounded: serde_json::Value,
    validity: serde_json::Value,
) -> Result<DeclarativeLimit, DslError> {
    let t = |name: &str| json!({"name": name, "kind": "absolute_temperature", "units": ["K"]});
    let def = json!({
        "id": "test",
        "title": "test",
        "direction": "max",
        "bounded_dimension": bounded,
        "parameters": [t("Th"), t("Tc"), t("T"), {"name": "P", "dimension": "W", "units": ["W", "kW", "MW"]}],
        "formula": formula,
        "validity": validity,
        "epistemic_status": "theorem"
    });
    DeclarativeLimit::from_json(&def.to_string())
}

#[test]
fn carnot_one_minus_tc_over_th_is_dimensionless() {
    assert!(compile("1 - Tc/Th", json!("dimensionless")).is_ok());
    assert!(compile("1 - Tc/Th", json!([0, 0, 0, 0, 0, 0, 0])).is_ok());
}

#[test]
fn tc_minus_th_declared_dimensionless_is_a_dimension_mismatch() {
    assert_eq!(
        compile("Tc - Th", json!("dimensionless")).map(|_| ()),
        Err(DslError::DimensionMismatch {
            context: "formula".into(),
            computed: [0, 0, 0, 0, 1, 0, 0],
            declared: [0; 7],
        })
    );
}

#[test]
fn kb_times_t_has_the_dimension_of_energy() {
    assert!(compile("kB*T", json!("J")).is_ok());
    assert!(compile("kB*T", json!([2, 1, -2, 0, 0, 0, 0])).is_ok());
    assert!(matches!(
        compile("kB*T", json!("W")),
        Err(DslError::DimensionMismatch {
            computed: [2, 1, -2, 0, 0, 0, 0],
            declared: [2, 1, -3, 0, 0, 0, 0],
            ..
        })
    ));
}

#[test]
fn ln_and_sqrt_of_a_temperature_fail() {
    assert!(matches!(
        compile("ln(T)", json!("dimensionless")),
        Err(DslError::DimensionlessArgumentRequired { function: "ln", .. })
    ));
    assert!(matches!(
        compile("exp(T)", json!("dimensionless")),
        Err(DslError::DimensionlessArgumentRequired {
            function: "exp",
            ..
        })
    ));
    assert!(matches!(
        compile("sqrt(T)", json!("dimensionless")),
        Err(DslError::OddExponentUnderSqrt { .. })
    ));
    assert!(compile("ln(Th/Tc)", json!("dimensionless")).is_ok());
    assert!(compile("sqrt(T^2)/T", json!("dimensionless")).is_ok());
}

#[test]
fn addition_needs_equal_dimensions_and_celsius_is_never_implied() {
    assert!(matches!(
        compile("1 + Th", json!("dimensionless")),
        Err(DslError::IncompatibleOperands { .. })
    ));
    assert!(matches!(
        compile("(T + 273.15)/Th", json!("dimensionless")),
        Err(DslError::IncompatibleOperands { .. })
    ));
    assert!(matches!(
        compile("(Th + Tc)/T", json!("dimensionless")),
        Err(DslError::TemperatureKindMismatch { .. })
    ));
    assert!(compile("(Th - Tc)/T", json!("dimensionless")).is_ok());
}

#[test]
fn parse_errors_carry_their_position() {
    let position = |f: &str| match compile(f, json!("dimensionless")) {
        Err(DslError::Parse { position, .. }) => position,
        other => panic!("expected a parse error for {f}: {other:?}"),
    };
    assert_eq!(position("1 - * Tc"), 4);
    assert_eq!(position("(1 - Tc/Th"), 10);
    assert_eq!(position("Tc / Th ^ 0.5"), 10);
    assert_eq!(position("Tc = Th"), 3);
    assert_eq!(position("Tc/Th Th"), 6);
}

#[test]
fn an_unknown_name_is_an_unknown_symbol() {
    assert_eq!(
        compile("1 - Tx/Th", json!("dimensionless")).map(|_| ()),
        Err(DslError::UnknownSymbol {
            source_text: "1 - Tx/Th".into(),
            name: "Tx".into(),
            position: 4
        })
    );
    assert!(matches!(
        compile("log(Th/Tc)", json!("dimensionless")),
        Err(DslError::UnknownSymbol { name, .. }) if name == "log"
    ));
}

#[test]
fn constants_have_their_si_dimensions() {
    assert!(compile("h*c", json!([3, 1, -2, 0, 0, 0, 0])).is_ok());
    assert!(compile("e*NA", json!([0, 0, 1, 1, 0, -1, 0])).is_ok());
    assert!(compile("pi*ln2", json!("dimensionless")).is_ok());
}

#[test]
fn validity_conditions_are_checked_and_mapped_only_to_temperature_orders() {
    assert!(compile_with(
        "1 - Tc/Th",
        json!("dimensionless"),
        json!(["Tc > 0", "Th > Tc"])
    )
    .is_ok());
    assert!(matches!(
        compile_with("1 - Tc/Th", json!("dimensionless"), json!(["P > 0"])),
        Err(DslError::UnsupportedCondition { .. })
    ));
    assert!(matches!(
        compile_with("1 - Tc/Th", json!("dimensionless"), json!(["Th > P"])),
        Err(DslError::IncompatibleOperands { .. })
    ));
}

#[test]
fn a_temperature_unit_or_reserved_name_is_refused() {
    assert!(matches!(
        compile("Th", json!("K")),
        Err(DslError::InvalidDefinition { .. })
    ));
    let def = json!({
        "id": "x", "title": "x", "direction": "max", "bounded_dimension": "dimensionless",
        "parameters": [{"name": "c", "dimension": "dimensionless", "units": ["dimensionless"]}],
        "formula": "c", "epistemic_status": "theorem"
    });
    assert!(matches!(
        DeclarativeLimit::from_json(&def.to_string()),
        Err(DslError::InvalidDefinition { .. })
    ));
}
