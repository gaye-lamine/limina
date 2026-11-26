---

inclusion: always

---

# Tech stack

- Rust workspace. Typed physical quantities via an in-house runtime Dimension type (SI exponents). uom was rejected: compile-time checks only. Property-based tests with `proptest`.

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

- thiserror = "=2.0.21"
- proptest = "=1.12.0" (dev)
- web: primeng 21.1.10, @primeuix/themes 2.0.3, pdfmake 0.3.15 (dev: @types/pdfmake 0.3.5)
- web: tailwindcss 4.3.5, @tailwindcss/postcss 4.3.5, postcss 8.5.33, @angular/cdk 21.2.14, lucide-angular 1.0.0, @fontsource-variable/inter 5.3.0, @fontsource-variable/jetbrains-mono 5.3.0; dev: @playwright/test 1.65.1, @types/node 20.19.43 (see docs/design-system.md)
- Web e2e: `npx playwright test` in web/ (app on 4200 and API on 3000 running; uses installed Edge)
