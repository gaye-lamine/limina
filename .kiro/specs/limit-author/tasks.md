# Tasks: limit-author

Status: v1, frozen. Checks once per group: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

## Group 1: inverse questions (limina-dsl)

- [x] **T01** `solve.rs`, `Registry::builtin()`, `Serialize` on definitions, `GRAMMAR`.
- [x] **T02** Oracle tests and properties (Carnot, COP heating, COP cooling, wrong side, non-monotone, no range).

## Group 2: authoring (limina-extract)

- [x] **T03** `author.rs`: prompt, loop with at most 2 repairs, overrides, typed errors.
- [x] **T04** Mock tests, injection and key tests.

## Group 3: API

- [x] **T05** In-memory authored limits in `GET /v1/limits` and `POST /v1/evaluate`, draft sentences.
- [x] **T06** `POST /v1/limits/author`, `POST /v1/solve`, error codes, handler tests (Landauer).

## Group 4: delivery

- [x] **T07** Live smoke test, README, `.gitignore`, commit, push, CI.
