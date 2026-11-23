# Requirements: api-evaluate
Amended by api-limit-metadata (v1): `GET /v1/limits` gains `claim_unit` and `parameters[].units`.

Status: v1, frozen. Based on `limina-core` (decision-engine v5, limit-betz v1, limit-carnot v1). `limina-core` is not modified by this spec.

## Scope

A new crate `crates/limina-api` exposing the deterministic engine over HTTP. No LLM, no database, no authentication. Only the limits `betz` and `carnot` are served.

## Requirements

- R-01 `GET /healthz` returns 200 and `{"status":"ok"}`.
- R-02 `GET /v1/limits` returns the limits `betz` and `carnot`, each with `id`, `direction` (`Max` or `Min`), `assumptions` (id and option ids) and `parameters` (id, kind, required).
- R-03 `POST /v1/evaluate` takes `limit_id` (`betz` or `carnot`), `claim {value, unit, uncertainty?}`, `constraints` (assumption id to allowed option ids) and `parameters` (name to `{value, unit}`). `constraints` and `parameters` default to empty.
- R-04 The response contains `verdict` (`IMPOSSIBLE`, `CONDITIONAL`, `COMPATIBLE`, `INSUFFICIENT_DATA`), `limit_id`, `engine_version`, `wording`, `bounds` (permissive and restrictive `b_min`, `b_max`, and `margin` when available), `configurations` (the members of C*, each tagged `Certain` or `Depends` with the parameters it depends on) and `missing_parameters` (each tagged `NoDomain` or `HasDomain`).
- R-05 The verdict is the verdict returned by `limina_core::decide`. The API never changes it.
- R-06 `wording` uses only these formulas:
  - IMPOSSIBLE: `inconsistent with limit X under assumptions Y`
  - CONDITIONAL: `possible only if ...`
  - COMPATIBLE: `does not exceed the bound; this does not prove the performance is reached`
  - INSUFFICIENT_DATA: `to decide, the following is needed: ...`

  `Y` lists the assumptions and the options left in the configuration set by the request constraints. The words fraud, lie, scam, hoax never appear in any response.
- R-07 Every `EngineError` maps to HTTP 422 with `{"error":{"code","message"}}`. The code is the name of the `EngineError` variant (for example `NotApplicable`, `InvalidTemperatureOrder`) and is stable. The match over variants has no wildcard arm.
- R-08 An unknown `limit_id` gives 422 with code `UnknownLimit`.
- R-09 A body that is not valid JSON, or does not match the request schema, gives 400 with code `InvalidJson`.
- R-10 Claim and parameter values given with unit `K` become absolute temperatures when the limit expects an absolute temperature. The API converts no other unit and never converts Celsius.
- R-11 The DTOs live in the API crate. No dependency on an LLM crate, a database or an auth library.

## Exact dependency versions

| Crate | Version | Use |
|---|---|---|
| axum | =0.8.9 | HTTP routing |
| tokio | =1.54.1 | runtime (features `macros`, `rt-multi-thread`, `net`) |
| serde | =1.0.229 | DTOs (feature `derive`) |
| serde_json | =1.0.154 | JSON parsing of the request body |
| thiserror | =2.0.21 | typed errors |
| tower | =0.5.3 (dev) | `ServiceExt::oneshot` in handler tests |
| http-body-util | =0.1.5 (dev) | reading response bodies in tests |
| proptest | =1.12.0 (dev) | fixed-seed property tests through the API |

## Known limits

- `engine_version` is the version of the `limina-api` crate, because `limina-core` exposes no version constant. The two crates are released together.
- With Betz and Carnot, CONDITIONAL cannot occur: each assumption option gives the same bound or no bound. The CONDITIONAL path is tested with a fake limit in unit tests.
- The verdict IMPOSSIBLE holds for the configurations allowed by the `constraints` sent by the caller. The API cannot check that they match the source text.
