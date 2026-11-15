//! P-05 (unit round trip) and P-06 (incompatible dimension is a typed error).
//!
//! The expected unit table is copied from design.md 3.4. The tolerance check
//! is written here with the constants of D-06; it does not call the crate's
//! `approx_eq`.

mod common;

use std::collections::BTreeMap;

use common::scenario::config;
use common::TestLimit;
use limina_core::{
    decide, AbsoluteTemperature, Claim, ConstraintsMap, DecisionConfig, DecisionInput, Dimension,
    Direction, EngineError, Measure, MeasureKind, Quantity, TemperatureDifference,
};
use proptest::prelude::*;

/// Plain units of design 3.4 (all except `K`), with exponents and factor.
const PLAIN_UNITS: [(&str, [i8; 7], f64); 12] = [
    ("W", [2, 1, -3, 0, 0, 0, 0], 1.0),
    ("kW", [2, 1, -3, 0, 0, 0, 0], 1e3),
    ("MW", [2, 1, -3, 0, 0, 0, 0], 1e6),
    ("J", [2, 1, -2, 0, 0, 0, 0], 1.0),
    ("kWh", [2, 1, -2, 0, 0, 0, 0], 3.6e6),
    ("m", [1, 0, 0, 0, 0, 0, 0], 1.0),
    ("s", [0, 0, 1, 0, 0, 0, 0], 1.0),
    ("kg", [0, 1, 0, 0, 0, 0, 0], 1.0),
    ("mol", [0, 0, 0, 0, 0, 1, 0], 1.0),
    ("A", [0, 0, 0, 1, 0, 0, 0], 1.0),
    ("cd", [0, 0, 0, 0, 0, 0, 1], 1.0),
    ("dimensionless", [0, 0, 0, 0, 0, 0, 0], 1.0),
];

/// D-06 tolerance, written independently of the crate.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 + 1e-9 * a.abs().max(b.abs())
}

/// Finite values in `[-1e12, 1e12]`, zero included.
fn value() -> impl Strategy<Value = f64> {
    prop_oneof![Just(0.0), -1e12..=1e12f64]
}

fn dims_differ(a: usize, b: usize) -> bool {
    PLAIN_UNITS[a].1 != PLAIN_UNITS[b].1
}

/// One-configuration limit bounded in the given kind.
fn limit_of_kind(kind: MeasureKind) -> TestLimit {
    let mut limit = TestLimit::new(Direction::Max)
        .with_assumption("a", &["0"])
        .with_bound(&["0"], 1.0, 1.0);
    limit.kind = kind;
    limit
}

fn input(value: Measure, uncertainty: Option<Measure>) -> DecisionInput {
    DecisionInput {
        claim: Claim { value, uncertainty },
        config: DecisionConfig::default(),
        constraints: ConstraintsMap::new(),
        known_parameters: BTreeMap::new(),
    }
}

fn plain(x: f64, unit: &str) -> Measure {
    Measure::Plain(Quantity::from_unit(x, unit).expect("valid quantity"))
}

#[test]
fn unit_table_rows_match_design_3_4() {
    for (symbol, exponents, factor) in PLAIN_UNITS {
        let q = Quantity::from_unit(1.0, symbol).expect("unit from the design table");
        assert_eq!(q.si_value(), factor, "factor of {symbol}");
        assert_eq!(
            q.dimension(),
            Dimension::new(exponents),
            "dimension of {symbol}"
        );
    }
    // `K` exists but only through the temperature types.
    assert_eq!(
        Quantity::from_unit(1.0, "K"),
        Err(EngineError::TemperatureKindRequired)
    );
}

proptest! {
    #![proptest_config(config())]

    /// P-05: value -> unit -> value returns the original within D-06,
    /// compared in SI base units.
    #[test]
    fn p05_round_trip_returns_the_original_value(i in 0usize..12, x in value()) {
        let (symbol, _, factor) = PLAIN_UNITS[i];
        let q = Quantity::from_unit(x, symbol).expect("finite value");
        let back = q.to_unit(symbol).expect("same unit");
        prop_assert!(close(back * factor, x * factor), "{symbol}: {x:?} -> {back:?}");
        prop_assert!(close(q.si_value(), x * factor), "{symbol}: SI {:?}", q.si_value());
    }

    /// P-05 for temperatures: kelvin in, kelvin out, exactly.
    #[test]
    fn p05_temperature_round_trip(k in 0.0..=1e6f64, d in -1e6..=1e6f64) {
        let t = AbsoluteTemperature::from_kelvin(k).expect("valid");
        prop_assert_eq!(t.kelvin(), k);
        let dt = TemperatureDifference::from_kelvin(d).expect("valid");
        prop_assert_eq!(dt.kelvin(), d);
    }

    /// P-06: a claim of another dimension than the bounded quantity is a
    /// typed error, never a verdict.
    #[test]
    fn p06_claim_of_another_dimension_is_a_typed_error(
        claim_unit in 0usize..12,
        limit_unit in 0usize..12,
        x in value(),
    ) {
        prop_assume!(dims_differ(claim_unit, limit_unit));
        let kind = MeasureKind::Plain(Dimension::new(PLAIN_UNITS[limit_unit].1));
        let result = decide(
            &limit_of_kind(kind),
            &input(plain(x, PLAIN_UNITS[claim_unit].0), None),
        );
        let is_incompatible = matches!(result, Err(EngineError::IncompatibleDimension { .. }));
        prop_assert!(is_incompatible, "got {:?}", result);
    }

    /// P-06: an uncertainty of another dimension than the value is a typed
    /// error.
    #[test]
    fn p06_uncertainty_of_another_dimension_is_a_typed_error(
        value_unit in 0usize..12,
        u_unit in 0usize..12,
        x in value(),
        u in 0.0..=1e6f64,
    ) {
        prop_assume!(dims_differ(value_unit, u_unit));
        let kind = MeasureKind::Plain(Dimension::new(PLAIN_UNITS[value_unit].1));
        let result = decide(
            &limit_of_kind(kind),
            &input(
                plain(x, PLAIN_UNITS[value_unit].0),
                Some(plain(u, PLAIN_UNITS[u_unit].0)),
            ),
        );
        let is_incompatible = matches!(result, Err(EngineError::IncompatibleDimension { .. }));
        prop_assert!(is_incompatible, "got {:?}", result);
    }

    /// P-06: converting to a unit of another dimension is a typed error.
    #[test]
    fn p06_conversion_to_another_dimension_is_a_typed_error(
        from in 0usize..12,
        to in 0usize..12,
        x in value(),
    ) {
        prop_assume!(dims_differ(from, to));
        let q = Quantity::from_unit(x, PLAIN_UNITS[from].0).expect("finite value");
        let is_incompatible = matches!(
            q.to_unit(PLAIN_UNITS[to].0),
            Err(EngineError::IncompatibleDimension { .. })
        );
        prop_assert!(is_incompatible);
    }

    /// P-06: an absolute temperature against a temperature difference is
    /// `TemperatureKindMismatch`, as claim and as uncertainty.
    #[test]
    fn p06_absolute_against_difference_is_a_kind_mismatch(k in 0.0..=1e4f64) {
        let abs_t = Measure::AbsoluteTemperature(AbsoluteTemperature::from_kelvin(k).expect("valid"));
        let diff_t = Measure::TemperatureDifference(TemperatureDifference::from_kelvin(k).expect("valid"));

        let claim_vs_limit = decide(
            &limit_of_kind(MeasureKind::TemperatureDifference),
            &input(abs_t, None),
        );
        prop_assert_eq!(claim_vs_limit, Err(EngineError::TemperatureKindMismatch));

        let abs_as_uncertainty = decide(
            &limit_of_kind(MeasureKind::AbsoluteTemperature),
            &input(abs_t, Some(abs_t)),
        );
        prop_assert_eq!(abs_as_uncertainty, Err(EngineError::TemperatureKindMismatch));

        let plain_vs_temperature = decide(
            &limit_of_kind(MeasureKind::AbsoluteTemperature),
            &input(plain(k, "m"), None),
        );
        let is_incompatible = matches!(
            plain_vs_temperature,
            Err(EngineError::IncompatibleDimension { .. })
        );
        prop_assert!(is_incompatible);

        let diff_vs_absolute_limit = decide(
            &limit_of_kind(MeasureKind::AbsoluteTemperature),
            &input(diff_t, None),
        );
        prop_assert_eq!(diff_vs_absolute_limit, Err(EngineError::TemperatureKindMismatch));
    }
}
