//! Unit table: the only unit symbols the engine accepts (design 3.4).
//!
//! Lookup is an exact symbol match on this table. Since D-25, a symbol not
//! in the table is parsed as a prefixed or compound SI unit by
//! `units_parse`; anything else is rejected with `UnknownUnit`. The table
//! itself and `units_for_dimension` are unchanged.

use crate::dimension::Dimension;
use crate::error::EngineError;

/// One row of the unit table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct UnitEntry {
    /// Exact unit symbol.
    pub symbol: &'static str,
    /// Dimension of the unit.
    pub dimension: Dimension,
    /// Multiply a value in this unit by this factor to get SI base units.
    pub factor_to_si: f64,
}

const POWER: Dimension = Dimension::new([2, 1, -3, 0, 0, 0, 0]);
const ENERGY: Dimension = Dimension::new([2, 1, -2, 0, 0, 0, 0]);

/// The 13 accepted units, from design 3.4. `K` is only usable through the
/// temperature types.
const UNIT_TABLE: [UnitEntry; 13] = [
    entry("W", POWER, 1.0),
    entry("kW", POWER, 1e3),
    entry("MW", POWER, 1e6),
    entry("J", ENERGY, 1.0),
    entry("kWh", ENERGY, 3.6e6),
    entry("K", Dimension::new([0, 0, 0, 0, 1, 0, 0]), 1.0),
    entry("m", Dimension::new([1, 0, 0, 0, 0, 0, 0]), 1.0),
    entry("s", Dimension::new([0, 0, 1, 0, 0, 0, 0]), 1.0),
    entry("kg", Dimension::new([0, 1, 0, 0, 0, 0, 0]), 1.0),
    entry("mol", Dimension::new([0, 0, 0, 0, 0, 1, 0]), 1.0),
    entry("A", Dimension::new([0, 0, 0, 1, 0, 0, 0]), 1.0),
    entry("cd", Dimension::new([0, 0, 0, 0, 0, 0, 1]), 1.0),
    entry("dimensionless", Dimension::DIMENSIONLESS, 1.0),
];

const fn entry(symbol: &'static str, dimension: Dimension, factor_to_si: f64) -> UnitEntry {
    UnitEntry {
        symbol,
        dimension,
        factor_to_si,
    }
}

/// Returns the table row for `symbol`, or `UnknownUnit` if the symbol is not
/// in the table. The match is exact and case-sensitive.
pub(crate) fn lookup_unit(symbol: &str) -> Result<UnitEntry, EngineError> {
    UNIT_TABLE
        .iter()
        .find(|e| e.symbol == symbol)
        .copied()
        .ok_or_else(|| EngineError::UnknownUnit(symbol.to_string()))
}

/// The dimension and factor of `symbol`: the table row when there is one,
/// else the parsed prefixed or compound SI unit (D-25).
pub(crate) fn resolve_unit(symbol: &str) -> Result<crate::units_parse::UnitInfo, EngineError> {
    match lookup_unit(symbol) {
        Ok(e) => Ok(crate::units_parse::UnitInfo {
            dimension: e.dimension,
            factor_to_si: e.factor_to_si,
        }),
        Err(_) => crate::units_parse::parse_unit(symbol),
    }
}

/// Unit symbols of the table whose dimension is `dimension`, in table order.
///
/// The list is empty when no table unit has that dimension. Every returned
/// symbol is accepted by `Quantity::from_unit`, except that `K` is only
/// usable through the temperature types where the limit expects one.
/// Reference: unit table, decision-engine design 3.4.
pub fn units_for_dimension(dimension: Dimension) -> Vec<&'static str> {
    UNIT_TABLE
        .iter()
        .filter(|e| e.dimension == dimension)
        .map(|e| e.symbol)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{lookup_unit, UNIT_TABLE};
    use crate::dimension::Dimension;
    use crate::error::EngineError;

    /// Expected rows, copied from design 3.4.
    const EXPECTED: [(&str, [i8; 7], f64); 13] = [
        ("W", [2, 1, -3, 0, 0, 0, 0], 1.0),
        ("kW", [2, 1, -3, 0, 0, 0, 0], 1e3),
        ("MW", [2, 1, -3, 0, 0, 0, 0], 1e6),
        ("J", [2, 1, -2, 0, 0, 0, 0], 1.0),
        ("kWh", [2, 1, -2, 0, 0, 0, 0], 3.6e6),
        ("K", [0, 0, 0, 0, 1, 0, 0], 1.0),
        ("m", [1, 0, 0, 0, 0, 0, 0], 1.0),
        ("s", [0, 0, 1, 0, 0, 0, 0], 1.0),
        ("kg", [0, 1, 0, 0, 0, 0, 0], 1.0),
        ("mol", [0, 0, 0, 0, 0, 1, 0], 1.0),
        ("A", [0, 0, 0, 1, 0, 0, 0], 1.0),
        ("cd", [0, 0, 0, 0, 0, 0, 1], 1.0),
        ("dimensionless", [0, 0, 0, 0, 0, 0, 0], 1.0),
    ];

    #[test]
    fn every_row_matches_the_design_table() {
        for (symbol, exponents, factor) in EXPECTED {
            let e = lookup_unit(symbol).expect("symbol from the design table");
            assert_eq!(e.symbol, symbol);
            assert_eq!(
                e.dimension,
                Dimension::new(exponents),
                "dimension of {symbol}"
            );
            assert_eq!(e.factor_to_si, factor, "factor of {symbol}");
        }
    }

    #[test]
    fn table_has_exactly_13_distinct_symbols() {
        assert_eq!(UNIT_TABLE.len(), 13);
        for (i, a) in UNIT_TABLE.iter().enumerate() {
            for b in &UNIT_TABLE[i + 1..] {
                assert_ne!(a.symbol, b.symbol);
            }
        }
    }

    #[test]
    fn unknown_symbols_are_rejected() {
        // Table lookup only; since D-25 "W/m^2", "GW" and "Wh" are parsed by
        // `resolve_unit` instead (see units_parse.rs).
        for symbol in ["kw", "w", "W/m^2", "°C", "degC", "GW", "Wh", "", " W"] {
            assert_eq!(
                lookup_unit(symbol),
                Err(EngineError::UnknownUnit(symbol.to_string())),
                "symbol {symbol:?}"
            );
        }
    }
}
