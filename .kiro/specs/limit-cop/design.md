# Design: limit-cop

Status: v1, frozen. Based on `requirements.md` v1, `decision-engine` v5 and `limit-carnot` v1. No new dependency, no new error variant, `engine.rs` untouched.

## 1. Files

- New: `crates/limina-core/src/limits/cop.rs`, `crates/limina-core/tests/limit_cop.rs`.
- Modify: `limits/mod.rs` (declare `cop`), `lib.rs` (re-export `Cop`).
- Modify: `crates/limina-api/src/registry.rs` (serve `cop`, display strings), API tests.
- Modify: `web/src/app/examples.ts`, `strings.ts`, result panel if needed, component tests.

## 2. The limit

- `id()` = `"cop"`. `direction()` = `Max`. `bounded_kind()` = `Plain(DIMENSIONLESS)`.
- `assumptions()` = `mode` with options `cooling` and `heating` (declaration order: `heating`, `cooling`; the engine orders configurations lexicographically), values `None`.
- `parameters()` = `Th`, `Tc`, `AbsoluteTemperature`, required, `plausible: None`. Built once in `Cop::new()`.
- `bound(config, params)`:
  - Unknown `mode` option: `ConstraintOutsideDomain`. Missing `mode`: `UnknownAssumption`. Missing parameter: `UnknownParameter` (defensive; `decide` reports missing parameters first).
  - Valid only if `Tc.lo > 0` and `Th.lo > Tc.hi`, else `InvalidTemperatureOrder { limit_id: "cop" }`. A typed error is never a verdict.
  - Both bounds are decreasing in `Th` and increasing in `Tc` on the valid domain, so the exact range over the enclosures is taken at the corners:
    - heating: `[Th.hi/(Th.hi-Tc.lo), Th.lo/(Th.lo-Tc.hi)]`;
    - cooling: `[Tc.lo/(Th.hi-Tc.lo), Tc.hi/(Th.lo-Tc.hi)]`.
  - For point enclosures this is the f64 value of `Th/(Th-Tc)` or `Tc/(Th-Tc)`. Correctly rounded subtraction and division are monotone, so `lo <= hi` holds in f64.

## 3. Behaviour through `decide`

With 300 K / 280 K, `S` has `cooling` (14.0) and `heating` (15.0), so `B_min = 14`, `B_max = 15`. A claim in `(14 + g, 15]` is CONDITIONAL with `C* = {heating}`, Certain (no missing parameter). Constraining `mode` reduces `S` to one configuration.

## 4. API

`registry.rs` adds `Cop` to `find_limit` and to the listing, after `carnot`. Display strings: title "Carnot COP limit", formula "COP <= Th/(Th-Tc) (heating), Tc/(Th-Tc) (cooling)", one plain statement. Same fields as the other limits; nothing else changes.

## 5. Web

One example appended to `EXAMPLES`: limit `cop`, claim `14.5`, `Th` 300, `Tc` 280, no constraint. The result panel already lists configurations with their tag; it is made clearer by showing each configuration as "mode = heating" with its Certain or Depends tag under a "Satisfying configurations" heading.

## 6. Tests

Golden tests in `cop.rs` from the acceptance values. Engine examples and properties in `tests/limit_cop.rs` with the fixed-seed runner; the oracles are written in the test. Handler tests in `crates/limina-api/tests/evaluate.rs`. Component test in `web/`.
