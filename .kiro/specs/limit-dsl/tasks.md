# Tasks: limit-dsl

Status: v1, frozen. Checks once per group: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Existing crates and specs are not modified.

## Group 1: language

- [x] **T01** `definition.rs`, `error.rs`.
- [x] **T02** `expr.rs`: lexer, parser, positions, `UnknownSymbol`.
- [x] **T03** `analysis.rs`: dimensions and temperature kinds; `ivops.rs`.

## Group 2: limits

- [x] **T04** `limit.rs`: `DeclarativeLimit` implementing `Limit`.
- [x] **T05** `report.rs`, `registry.rs`, `data/limits/{betz,carnot,cop}.json`.
- [x] **T06** Tests: dimension examples, parser errors, conservativeness, carnot monotonicity, draft never validated, differential properties; README section.
