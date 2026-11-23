# Requirements: limit-author

Status: v1, frozen. Base: `limit-dsl` v1, `extract-claims` v1, `api-evaluate` v1 as amended by `api-limit-metadata` v1 (all unchanged). `limina-core` is not modified. `limina-dsl` gets a new module; `limina-extract` and `limina-api` get additions only: existing endpoints, fields, tests and contracts do not change.

## Summary

Two capabilities on top of the DSL. (1) Inverse questions: for which value of one parameter does the bound equal the claim? (2) A limit written from a sentence: a language model proposes a DSL definition, the DSL checks it, the model may repair it twice, and the result is a draft limit the engine can run. The verdict is always computed by the engine.

## Requirements

- **R-01 `solve_for`** (limina-dsl): given a limit, a claim, fixed parameters and constraints and one free parameter, find by bisection on the DSL evaluation the value of the free parameter where the bound equals the claim. Search range: the declared plausible domain, else `test_range`, else `NoSearchRange`.
- **R-02** Monotonicity is established first, with the sampling and trend rules of the check report, on the same range with the other parameters at their fixed values; non-monotone or undetermined gives `NotMonotone` and no threshold.
- **R-03** Result: threshold, direction ("at least" or "at most"), the engine verdict just below and just above the threshold, and the sentence "Not inconsistent with limit X only if P is at least/at most V (other parameters fixed at ...)". No banned word.
- **R-04 `author_limit`** (limina-extract): one system prompt built at runtime from the DSL grammar, the constants, the unit table and the definition schema; the text is delimited data. The model's JSON is parsed and compiled by the DSL; on any typed error the exact error goes back to the model, at most 2 repairs. Returned: definition, attempts, every error seen, check report. Still invalid: a typed error with the last error and no limit.
- **R-05** An authored limit always has status `draft`, an id starting with `draft-`, no `validated_by` (never set by code), and the reference "to be added".
- **R-06 API**: `POST /v1/limits/author {"text"}` (at most 2000 characters) authors and registers the limit in the running server, in memory only; 503 `ExtractionUnavailable` without a key. Authored limits appear in `GET /v1/limits` with `"status": "draft"` and work with `POST /v1/evaluate`; their IMPOSSIBLE sentence is "inconsistent with draft limit X under assumptions Y" and every other sentence starts with "draft limit X: ". `POST /v1/solve` returns the `solve_for` result; 422 with a stable code on `NotMonotone` and `NoSearchRange`.

## Acceptance criteria

1. Oracles: Carnot free Th, `Th = Tc/(1-c)`; COP heating `Th = c Tc/(c-1)`; COP cooling `Th = Tc (1+c)/c`. Properties (fixed seed, 256 cases, 1024 with `CI`): the threshold matches within 1e-9 relative; IMPOSSIBLE just on the wrong side beyond the guard, not IMPOSSIBLE on the right side; a non-monotone limit gives `NotMonotone`.
2. Authoring with a mock: correct at once; repaired on the second attempt with the error in the second request; never repaired gives the typed error; the model cannot set `validated` or `validated_by`; injected text changes nothing; the key never appears.
3. Handlers: a Landauer-type authored limit (`kB*T*ln2`, min, T absolute, energy) at 300 K: a claim 100 times the bound is COMPATIBLE, a claim below it IMPOSSIBLE.
