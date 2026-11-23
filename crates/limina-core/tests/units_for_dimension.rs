//! `units_for_dimension` returns the unit table of design.md 3.4, grouped by dimension.

use limina_core::{units_for_dimension, Dimension, EngineError, Quantity};

/// Expected symbols per dimension, in table order (design.md 3.4).
const EXPECTED: [([i8; 7], &[&str]); 9] = [
    ([2, 1, -3, 0, 0, 0, 0], &["W", "kW", "MW"]),
    ([2, 1, -2, 0, 0, 0, 0], &["J", "kWh"]),
    ([0, 0, 0, 0, 1, 0, 0], &["K"]),
    ([1, 0, 0, 0, 0, 0, 0], &["m"]),
    ([0, 0, 1, 0, 0, 0, 0], &["s"]),
    ([0, 1, 0, 0, 0, 0, 0], &["kg"]),
    ([0, 0, 0, 0, 0, 1, 0], &["mol"]),
    ([0, 0, 0, 1, 0, 0, 0], &["A"]),
    ([0, 0, 0, 0, 0, 0, 1], &["cd"]),
];

#[test]
fn symbols_match_the_design_table_for_each_dimension() {
    for (exponents, symbols) in EXPECTED {
        assert_eq!(
            units_for_dimension(Dimension::new(exponents)),
            symbols,
            "dimension {exponents:?}"
        );
    }
    assert_eq!(
        units_for_dimension(Dimension::DIMENSIONLESS),
        ["dimensionless"]
    );
}

#[test]
fn a_dimension_without_unit_gives_an_empty_list() {
    assert!(units_for_dimension(Dimension::new([3, 0, 0, 0, 0, 0, 0])).is_empty());
}

#[test]
fn every_returned_symbol_is_accepted_by_from_unit_with_that_dimension() {
    let all = EXPECTED
        .iter()
        .map(|(e, _)| Dimension::new(*e))
        .chain([Dimension::DIMENSIONLESS]);
    for dimension in all {
        for symbol in units_for_dimension(dimension) {
            if symbol == "K" {
                // `K` is only usable through the temperature types (units.rs).
                assert_eq!(
                    Quantity::from_unit(1.0, symbol),
                    Err(EngineError::TemperatureKindRequired)
                );
                continue;
            }
            let q = Quantity::from_unit(1.0, symbol).expect("symbol from the table");
            assert_eq!(q.dimension(), dimension, "unit {symbol}");
        }
    }
}
