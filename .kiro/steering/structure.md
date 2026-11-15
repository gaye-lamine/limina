---

inclusion: always

---

# Repository structure

crates/limina-core (units, intervals, Limit trait, decision engine, limits/)

crates/limina-wasm, limina-extract, limina-api, limina-mcp (added in later phases)

web/ (Angular app), data/ (reference dataset + schema + prereg), docs/ (journal.md, methodology.md)

.kiro/ (steering, specs, hooks, settings) — MUST be committed, never gitignored.

## Conventions

- One limit = one file crates/limina-core/src/limits/<id>.rs + one spec in .kiro/specs/ + golden tests.

- Small modules, small functions, no unwrap() in library code, typed errors with thiserror.

- Public items documented with the formula, assumptions, and reference.
