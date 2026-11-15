---

inclusion: always

---

# Tech stack

- Rust workspace. Typed physical quantities with the `uom` crate. Property-based tests with `proptest`.

- API: axum + tokio. Persistence: SQLite via sqlx. WASM build via wasm-bindgen.

- Web: Angular (strict TypeScript, signals, standalone components), D3 for plots, KaTeX for formulas, fast-check for in-browser property tests, Playwright for e2e.

- LLM access only through an `ExtractionProvider` trait. No provider-specific code outside its implementation.

- No secrets in the repo; read keys from environment variables.

- Pin exact dependency versions at scaffold time and record them below.

## Commands (fill after scaffold)

- Test: cargo test --workspace

- Lint: cargo clippy --workspace --all-targets -- -D warnings

- Format: cargo fmt --all --check

## Pinned versions

- uom = "=0.38.0"
- thiserror = "=2.0.21"
- proptest = "=1.12.0" (dev)
