# Tasks: physics-lint
Status: v1, frozen. Checks once per group: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. `limina-core` is not modified.

## Group 1: shared code and CLI
- [x] **T01** `analyze_text` and `solve_with` in `limina-api`, handlers call them.
- [x] **T02** Crate `limina-cli`: args, `.env`, positions, check, settle, inverse, render, run, main.
- [x] **T03** Demo document; mock tests (positions, exit codes, text, JSON, no key, key never printed, demo verdicts); live test `#[ignore]`.

## Group 2: Kiro and delivery
- [x] **T04** Steering `physics-claims.md`; hook `physics-lint` with the hook tool.
- [x] **T05** Live check on the demo, README "Physics lint", commit, push, CI.
