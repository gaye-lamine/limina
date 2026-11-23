# Tasks: limit-betz
Status: v1, frozen. Based on `requirements.md` v1 and `design.md` v1. Crate: `limina-core`.
Rules: after each task, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass. No `unwrap()` in library code. Public items documented. `engine.rs` and the decision-engine spec are not modified.

- [x] **T01 `NotApplicable` error.** Add `EngineError::NotApplicable { limit_id, assumption, option }` in `error.rs` and update its variant-list tests. Covers: R-03.
- [x] **T02 The limit (1).** Create `limits/mod.rs` and `limits/betz.rs`: `Betz` implementing `Limit` as in design 3. Export it from `lib.rs`. Covers: R-01, R-02, R-03.
- [x] **T03 Golden tests (2).** In `betz.rs`: `bound` equals 16/27 (within `approx_eq`) for `unshrouded_rotor` and `shrouded_total_capture_area`; `NotApplicable` for `shrouded_rotor_area_only`. Value 16/27 only. Covers: AC 1.
- [x] **T04 Example tests (3).** In `tests/limit_betz.rs`, through `decide`: 0.30 with `unshrouded_rotor` gives COMPATIBLE; 0.70 with `unshrouded_rotor` gives IMPOSSIBLE; 0.70 with `shrouded_total_capture_area` gives IMPOSSIBLE; any claim with `shrouded_rotor_area_only` gives `NotApplicable`; an unconstrained claim gives `NotApplicable`. Covers: AC 2 to 6.
- [x] **T05 Property test (4).** In `tests/limit_betz.rs`: for a claim in `[0, 16/27]` and any non-empty subset of the two valid options, `decide` never returns IMPOSSIBLE. Fixed seed, readable counterexamples. Covers: AC 7.
- [x] **T06 Public documentation (5).** Rustdoc on `Betz` with the formula, assumptions, the marked TODO reference, the instruction to constrain `reference_area`, the later mapping of `NotApplicable` to INSUFFICIENT_DATA ("reference area not stated"), and the shrouded-turbine note. Covers: R-04, R-05, R-06, AC 8.
