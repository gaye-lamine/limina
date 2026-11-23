# Design: limit-betz
Status: v1, frozen. Based on `requirements.md` v1 and `decision-engine` v5. Stack: Rust, `limina-core`, `thiserror`, `proptest`. No new dependency.

## 1. Overview
`Betz` is a unit struct implementing `Limit` in `crates/limina-core/src/limits/betz.rs`. `Limit::bound` already returns `Result<Interval, EngineError>` and `decide` propagates its errors (design of decision-engine, 5 and 6.4 step 5), so no engine change is needed. Checked in `limit.rs`: the trait signature allows the error.

## 2. Files
- New: `crates/limina-core/src/limits/mod.rs` (declares `betz`), `crates/limina-core/src/limits/betz.rs`.
- Modify: `crates/limina-core/src/lib.rs` (add `mod limits;` and `pub use limits::Betz;`).
- Modify: `crates/limina-core/src/error.rs`. `EngineError` has no `NotApplicable` variant today. Add `NotApplicable { limit_id: String, assumption: String, option: String }` with a message such as "limit {limit_id} not applicable: {assumption} = {option} is outside its validity domain", and extend the variant list and count in its tests. This is the only change outside the limit. `engine.rs` is untouched.

## 3. The limit
- `id()` = `"betz"`. `direction()` = `Max`. `bounded_kind()` = `MeasureKind::Plain(Dimension::DIMENSIONLESS)`. `parameters()` = empty.
- `assumptions()` = one `Assumption` `reference_area`, options in this order: `unshrouded_rotor`, `shrouded_total_capture_area`, `shrouded_rotor_area_only`. Option `value` is `None`. The struct stores the `Vec<Assumption>` built in `Betz::new()`.
- `bound(config, params)` reads the option of `reference_area`:
  - `unshrouded_rotor` or `shrouded_total_capture_area`: `Interval::point(16.0 / 27.0)`.
  - `shrouded_rotor_area_only`: `Err(EngineError::NotApplicable { .. })`.
  - assumption absent from `config`: `Err(EngineError::UnknownAssumption("reference_area"))`.
  - any other option id: `Err(EngineError::ConstraintOutsideDomain { .. })`. Both cases are unreachable through `ConfigurationSet::build` and are kept as defensive typed errors, never `unwrap`.
- `params` is ignored. The constant `16.0 / 27.0` is the only number in the file.

## 4. Behaviour through `decide`
All errors are fatal for the call and returned to the caller. None is logged here (the crate does no I/O).
- Constraint to one or both valid options: all `B(c) = 16/27`. Claim above it by more than the guard gives IMPOSSIBLE, at or below gives COMPATIBLE (no uncertainty) or CONDITIONAL (uncertainty straddling the bound).
- Constraint containing `shrouded_rotor_area_only`, or no constraint: `S` contains it, so `decide` returns `NotApplicable`. No verdict is produced, so a false IMPOSSIBLE is impossible by construction. The caller (later, the extraction pipeline) maps it to INSUFFICIENT_DATA ("reference area not stated").
- Invariant owner: the limit owns the validity domain (it refuses the third option). The engine owns the verdict rules and the guard.

## 5. Documentation (public items)
Rustdoc on `Betz` states: formula `Cp <= 16/27` (about 0.593), from the actuator disk model; assumptions (incompressible, unconfined flow, `reference_area` with its three options); epistemic status model bound; `// TODO(reference): to be supplied by the author` and the same marked line in the doc comment; and the physical note that shrouded-turbine claims referenced to the rotor area alone can exceed 16/27, which is why `shrouded_rotor_area_only` returns `NotApplicable`. It also states that callers must constrain `reference_area`. Wording follows `product.md`: "inconsistent with limit X under assumptions Y".

## 6. Tests
Unit tests in `betz.rs` (golden), example and property tests in `crates/limina-core/tests/limit_betz.rs`, using proptest 1.12.0 with the fixed-seed runner of `tests/common/` as in decision-engine 9.1. Claims are `Measure::Plain` with unit `dimensionless`, default `DecisionConfig`, no uncertainty. The property generates claims in `[0, 16/27]` and non-empty subsets of the two valid options.
