# Design: limit-carnot
Status: v1, frozen. Based on `requirements.md` v1, `decision-engine` v5 and `limit-betz` v1. Stack: Rust, `limina-core`, `thiserror`, `proptest`. No new dependency.
## 1. Overview
`Carnot` is a unit struct implementing `Limit` in `crates/limina-core/src/limits/carnot.rs`. Missing-parameter handling already exists in the engine (D-04, D-16), so no engine change is needed.
## 2. Files
- New: `crates/limina-core/src/limits/carnot.rs`.
- Modify: `limits/mod.rs` (declare `carnot`), `lib.rs` (`pub use limits::Carnot;`).
- Modify: `error.rs`. Checked: no existing variant fits `Th <= Tc` (`InvalidBound` carries no data, the others concern assumptions, parameter names or configuration sets). Add ONE variant, `InvalidTemperatureOrder { limit_id: String }`, message such as "limit {limit_id} requires Th > Tc > 0". Cover it in the exhaustive-variants test and in the wording test of `error.rs`. This is the only change outside the limit. `engine.rs` is untouched.
## 3. The limit
- `id()` = `"carnot"`. `direction()` = `Max`. `bounded_kind()` = `MeasureKind::Plain(Dimension::DIMENSIONLESS)`.
- `parameters()` = `Th` and `Tc`, kind `MeasureKind::AbsoluteTemperature`, `required: true`, `plausible: None`. Built once in `Carnot::new()`.
- `assumptions()` = one `Assumption` `cycle_model` with one option `ideal_reversible_two_reservoirs`, value `None`.
- `bound(config, params)`:
  - Read `Th` and `Tc` from the enclosure. If either is absent, return `EngineError::UnknownParameter`. Unreachable through `decide`, which reports missing parameters first, and kept as a defensive typed error.
  - Valid only if the enclosures guarantee `Tc.lo > 0` and `Th.lo > Tc.hi`. Otherwise `Err(InvalidTemperatureOrder)`. A typed error is never a verdict, so a false IMPOSSIBLE cannot come from it.
  - Otherwise compute the bound from `1 - Tc/Th` (point interval when both are points, `1.0 - tc / th` in f64). The result is monotone in each argument, so interval extremes come from the enclosure endpoints.
  - A `cycle_model` option other than the declared one returns `ConstraintOutsideDomain`. No `unwrap`.
## 4. Behaviour through `decide`
- Both temperatures present and valid: the verdict rules of the engine apply (IMPOSSIBLE, CONDITIONAL, COMPATIBLE).
- A missing temperature: INSUFFICIENT_DATA with the missing parameters listed, since no plausible domain exists to enclose them.
- `Th <= Tc` or `Tc <= 0`: the typed error is returned to the caller. No verdict.
- Celsius: the limit only accepts `AbsoluteTemperature`. Conversion is the caller's explicit job.
## 5. Documentation (public items)
Rustdoc on `Carnot`: formula `eta <= 1 - Tc/Th`; assumption `cycle_model` (ideal reversible cycle between two reservoirs); validity `Th > Tc > 0` in kelvin; epistemic status theorem (second law of thermodynamics); no silent Celsius conversion; `// TODO(reference): to be supplied by the author`, repeated in the doc comment. Wording follows `product.md`: "inconsistent with limit X under assumptions Y".
## 6. Tests
Golden tests in `carnot.rs`. Example and property tests in `crates/limina-core/tests/limit_carnot.rs`, proptest 1.12.0 with the fixed-seed runner of `tests/common/`. Property oracles compute `1 - Tc/Th` independently in the test. Claims are `Measure::Plain` dimensionless, default `DecisionConfig`.
