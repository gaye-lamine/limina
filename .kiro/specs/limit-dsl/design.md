# Design: limit-dsl

Status: v1, frozen. Based on `requirements.md` v1. Crate `crates/limina-dsl`, dependencies `limina-core` (path), `serde =1.0.229`, `serde_json =1.0.154`, `thiserror =2.0.21`; dev `proptest =1.12.0`. No network, no HTTP, no language model.

## 1. Modules

- `definition.rs`: the JSON format, `deny_unknown_fields`. `bounded_dimension` and a parameter `dimension` are either seven SI exponents (L, M, T, I, Θ, N, J) or a unit symbol of the core table. A parameter has a `dimension` or a `kind` (`absolute_temperature`, `temperature_difference`), never both; its `units` must all be core units of that dimension.
- `expr.rs`: lexer and recursive-descent parser. Precedence: unary minus below `^`, so `-x^2` is `-(x^2)`; `^` takes an integer literal, optionally negative or parenthesized. Names resolve to a parameter, a constant or a function; anything else is `UnknownSymbol` with its position. Constant names are reserved and cannot name a parameter.
- `analysis.rs`: static types `Plain(Dimension)`, `Absolute`, `Difference`; rules of R-01. In a comparison, the literal `0` compares with any dimension.
- `ivops.rs`: interval negation, integer power (exact for even powers), `sqrt`, `ln`, `exp`, `abs`, `min`, `max` on the core `Interval`; domain errors are `EngineError::NonFiniteValue`.
- `limit.rs`: `DeclarativeLimit` compiled from a definition; implements `Limit`.
- `report.rs`: `CheckReport`.
- `registry.rs`: loads a directory, sorted by file name; duplicate ids are refused.
- `error.rs`: `DslError`.

## 2. `bound` order (matches the Rust limits)

1. For each assumption in order: its option in the configuration, else `UnknownAssumption`; an option outside the domain is `ConstraintOutsideDomain`.
2. The formula: the option's own formula, else the shared one. None: `NotApplicable` for that option (Betz `shrouded_rotor_area_only` has no formula). At most one assumption may carry per-option formulas.
3. Every declared parameter, in order, from the enclosure, else `UnknownParameter`.
4. Validity conditions, each guaranteed over the enclosures (`a > b` holds when `a.lo > b.hi`), else `InvalidTemperatureOrder`.
5. Option conditions of the selected options, else `NotApplicable`.
6. Interval evaluation of the formula.

## 3. Known gap of the core (recorded, not worked around)

The core has no generic error for a failed validity condition; its only one is `InvalidTemperatureOrder`. A limit-level validity condition is therefore accepted only if it compares absolute temperatures (with each other or with `0`); any other one is refused at load time with `DslError::UnsupportedCondition`. An option condition always maps to `NotApplicable`, which fits any condition. A generic variant would need a change to `limina-core`, out of scope here.

## 4. Interval conservativeness

Each occurrence of a variable is evaluated independently, so `x - x` on `[1, 2]` gives `[-1, 1]` and `Th/(Th-Tc)` gives a wider range than its true range. A wider bound range moves the engine away from IMPOSSIBLE (IMPOSSIBLE needs the claim above the largest bound), so this cannot create a false IMPOSSIBLE. On point enclosures the evaluation is exact and matches the Rust limits.

## 5. Check report

Samples: 41 points per parameter for monotonicity (others at the midpoint), a 9-point grid per parameter for finiteness, non-negativity and validity. A parameter without a plausible interval, finite domain or `test_range` is `Undetermined`. Points where a validity condition fails are counted, not evaluated.

## 6. Tests

Unit tests per module; `tests/dimension.rs`, `tests/report.rs`, `tests/registry.rs`; `tests/differential.rs` with proptest (fixed seed, 256 cases, 1024 with `CI`).
