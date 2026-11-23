# Design: limit-author

Status: v1, frozen. Based on `requirements.md` v1.

## 1. limina-dsl

- New module `solve.rs`. `SolveRequest { claim (SI), constraints, fixed (SI values, kelvin for temperatures), free }`.
- The bound as a function of the free parameter is the largest bound of the configuration set for a Max limit (IMPOSSIBLE needs the claim above it) and the smallest for a Min limit, each computed by `Limit::bound` on point enclosures.
- Monotonicity: the report's `samples` (41 points) and `trend` functions, made crate-visible, applied to that function on the search range with the other parameters fixed. Points where the bound is undefined (validity) are skipped, as in the report.
- Bisection between the two neighbouring samples where `bound - claim` changes sign, until the bracket cannot shrink in f64. No crossing in the range: `NoThresholdInRange`.
- Direction: for a Max limit, an increasing bound gives "at least", a decreasing one "at most"; reversed for a Min limit.
- Side points: the threshold times `1 - 1e-6` and `1 + 1e-6` (relative), each evaluated by `decide`.
- Additions: `Registry::builtin()` with the three files embedded (`include_str!`), `Serialize` on the definition types, `expr::GRAMMAR`.

## 2. limina-extract

- New dependency on `limina-dsl` (so on `limina-core`): the authoring module needs the DSL checks. The extraction module is unchanged and still never calls the engine.
- New module `author.rs`: `author_system_prompt()`, `author_limit(provider, text)` and `Extractor::author_limit(text)` (the latter also takes the rate-limit slot per model call). One retry on 429 or 5xx, as for extraction.
- Each answer: `finish_reason` length or empty content is an error; fences stripped, first JSON object; then `status` forced to `draft`, `validated_by` removed, `references` set to `["to be added"]`, the id lower-cased with `draft-` in front, each change recorded as an override; then the DSL schema and compile. Errors go back verbatim in the next user message, with the previous answer in its own delimited block.
- `AuthorError::NotAuthored { last_error, attempts, errors }` or `AuthorError::Extract(ExtractError)` for provider failures (not repaired).

## 3. limina-api

- `AppState` gets an in-memory list of authored limits. `GET /v1/limits` appends them with `status: "draft"` (absent for built-ins). `POST /v1/evaluate` looks up built-ins first, then authored limits; `response::build_draft` gives the draft sentences.
- `POST /v1/limits/author` and `POST /v1/solve`; solve looks up the DSL definition of a built-in (`Registry::builtin()`) or an authored limit. New stable codes, all 422: `LimitNotAuthored`, `NotMonotone`, `NoSearchRange`, `NoThresholdInRange`, `UnknownFreeParameter`, `MissingFixedParameter`.
- `LimitDto.title`, `formula`, `statement` become `String` (same JSON).
