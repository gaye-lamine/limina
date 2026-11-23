# Design: api-evaluate

Status: v1, frozen. Based on `requirements.md` v1.

## Files

- `crates/limina-api/Cargo.toml`: dependencies pinned as in the requirements.
- `src/lib.rs`: `router()` and module wiring.
- `src/main.rs`: binds `127.0.0.1:3000` (override with env `LIMINA_API_ADDR`) and serves `router()`.
- `src/dto.rs`: request and response types (serde).
- `src/error.rs`: `ApiError` (thiserror), code mapping, `IntoResponse`.
- `src/registry.rs`: the two limits, lookup by id, `GET /v1/limits` content.
- `src/convert.rs`: DTO to `Measure`, `ConstraintsMap`, parameters.
- `src/response.rs`: `Decision` to `EvaluateResponse`, wording.
- `src/handlers.rs`: the three handlers.
- `tests/health.rs`, `tests/limits.rs`, `tests/evaluate.rs`: handler tests (`tower::ServiceExt::oneshot`).

## Flow of POST /v1/evaluate

1. Parse the body bytes with `serde_json`; any failure gives `ApiError::InvalidJson` (400).
2. Look up the limit; unknown id gives `UnknownLimit` (422).
3. Convert the claim with the limit's `bounded_kind()` and each parameter with its `ParameterSpec.kind`. Unit `K` with an absolute-temperature kind gives `Measure::AbsoluteTemperature`; the uncertainty of such a claim is a `TemperatureDifference`. Any other unit goes through `Quantity::from_unit`. A parameter name the limit does not declare gives `EngineError::UnknownParameter`.
4. Build the `ConstraintsMap` and call `decide` with `DecisionConfig::default()`.
5. Rebuild the configuration set with `ConfigurationSet::build` to list the assumptions and options for the wording.
6. Map the `Decision` to the response. Errors become 422.

## Response mapping

- `verdict` string from the `Verdict` variant.
- `bounds`: present when `Decision.enclosures` is `Some`; each enclosure gives `b_min`, `b_max` and an optional `margin {low, high}`.
- `configurations`: members of C* for CONDITIONAL only; each has `assumptions` (id to option), `support` (`Certain` or `Depends`) and `depends_on`.
- `missing_parameters`: from INSUFFICIENT_DATA only.

## Wording

- IMPOSSIBLE: `inconsistent with limit <id> under assumptions <a = o; b in {o1, o2}>`.
- CONDITIONAL: `possible only if <config> or <config>`, each config written `a = o, b = o`.
- COMPATIBLE: fixed sentence of R-06.
- INSUFFICIENT_DATA: `to decide, the following is needed: <ids>`.

## Product-rule check

The API adds no path to IMPOSSIBLE. It is returned only when `decide` returns it, which needs IMPOSSIBLE under both enclosures. Engine errors (such as NotApplicable) are never turned into a verdict.
