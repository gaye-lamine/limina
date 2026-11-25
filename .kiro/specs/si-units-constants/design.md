# Design: si-units-constants
Status: v1, frozen.

- `limina-core/src/units_parse.rs`: recursive-descent parser. `product := factor ((* | · | . | space) factor | / factor)*`, `factor := atom exponent? | "(" product ")" exponent?`. `/` divides by the next factor or group only. Symbols without prefix are matched first (`min`, `h`, `d`, `au`, ...), then prefixable symbols, then prefix + prefixable symbol. Public `parse_unit` and `UnitInfo`.
- `limina-core/src/units.rs`: `resolve_unit` (table, then parser), used by `Quantity::from_unit` and `to_unit`. `lookup_unit`, the table and `units_for_dimension` are unchanged.
- `limina-core/src/constants.rs`: `CONSTANTS`, `constant`, `PhysicalConstant`, `CODATA_VERSION`.
- `limina-dsl/src/expr.rs`: `Constant::Physical(index)`, `Constant::all()`, `Constant::uncertainty()`; `Constant::ALL` keeps its seven entries. `ivops` evaluates a measured constant as an interval. `limit.rs` checks parameter units with `parse_unit`.
- `limina-extract/src/author.rs`: the authoring prompt lists every constant with its uncertainty, and the accepted unit grammar.
