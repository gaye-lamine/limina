# Tasks: extract-claims

Status: v1, frozen. Checks once per group: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Tests first for S-01 to S-03. `limina-core` and its specs are not modified.

## Group 1: crate limina-extract

- [x] **T01** Security tests first: sentinel key never in `Debug`, errors or prompts (S-01); untraceable values dropped (S-02); delimiters neutralized and schema strict under injection (S-03).
- [x] **T02** `config.rs`, `SecretKey`, `provider.rs`, `error.rs`.
- [x] **T03** `catalog.rs`, `prompt.rs`.
- [x] **T04** `parse.rs`: fences, first JSON object, strict schema, `ExtractionFailed`.
- [x] **T05** `check.rs`: traceability, offsets, units, constraints.
- [x] **T06** `extractor.rs`: cache, rate limit, retry; `openai.rs`.

## Group 2: API

- [x] **T07** Security tests first: capabilities with and without a key, sentinel key never in any response.
- [x] **T08** `AppState`, `router_with`, `evaluate_request` shared by both endpoints, `GET /v1/capabilities`.
- [x] **T09** `POST /v1/analyze` with the three end-to-end cases and the error codes; `main.rs`.
- [x] **T10** Live test `#[ignore]`; README "Run with extraction"; `.env` in `.gitignore`.
