# Tasks: api-evaluate

Status: v1, frozen. After each task: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Tests are written before the code of each task.

- [x] T1 Scaffold `crates/limina-api` with pinned dependencies, add it to the workspace, `router()` with `GET /healthz`. Test: `tests/health.rs`.
- [x] T2 `ApiError` with the exhaustive `EngineError` code mapping, 400 and 422 responses. Unit tests in `error.rs`.
- [x] T3 DTOs, registry and `GET /v1/limits`. Test: `tests/limits.rs`.
- [x] T4 Conversion, `decide`, response mapping and wording, `POST /v1/evaluate`. Tests: `tests/evaluate.rs` (the seven required cases and more) and unit tests in `response.rs` (CONDITIONAL with a fake limit, banned words).
- [x] T5 `main.rs`, README section "Run the API", live check with curl, report.
