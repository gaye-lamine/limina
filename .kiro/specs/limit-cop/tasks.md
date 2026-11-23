# Tasks: limit-cop

Status: v1, frozen. Based on `requirements.md` v1 and `design.md` v1. Checks once per group: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and in `web/` `npm run lint`, `npm test`. `engine.rs` and the other specs are not modified.

## Group 1: core

- [x] **T01 The limit.** `limits/cop.rs`: `Cop` implementing `Limit` as in design 2; declared in `limits/mod.rs`, re-exported from `lib.rs`. Covers: R-01, R-02, R-03.
- [x] **T02 Golden tests.** In `cop.rs`, the six values of AC 1. Covers: AC 1.
- [x] **T03 Engine examples.** In `tests/limit_cop.rs`, AC 2 and AC 3 through `decide`. Covers: AC 2, AC 3, R-04.
- [x] **T04 Properties.** In `tests/limit_cop.rs`: AC 4, AC 5, AC 6, fixed seed, readable counterexamples. Covers: AC 4 to 6.
- [x] **T05 Documentation.** Rustdoc on `Cop` with both formulas, the assumption, the validity domain and the marked TODO reference, plus a test that checks it. Covers: R-05.

## Group 2: API and web

- [x] **T06 API.** Serve `cop` with its metadata; handler tests of AC 7; metadata test. Covers: R-06, AC 7.
- [x] **T07 Web.** The COP example, the clearer configuration list, and the component test of AC 8. Covers: R-07, AC 8.
