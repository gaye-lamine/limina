# Tasks: web-evaluate

Status: v1, frozen. After each task: in `web/` `npm run lint` and `npm test`; at the root `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Tests are written before the code of each task.

- [x] T1 Scaffold `web/` (Angular 21.2.x pinned, Vitest, angular-eslint, dev proxy, `.gitignore`), remove logo, favicon and placeholder page.
- [x] T2 `strings.ts`, `api.models.ts`, `ApiService` with error mapping. Test: `api.service.spec.ts`.
- [x] T3 `ResultPanel` with the four verdict styles and the error block. Test: `result-panel.spec.ts`.
- [x] T4 `LimitsList` and `EvaluateForm` (adapts to the limit, builds the request). Tests: `limits-list.spec.ts`, `evaluate-form.spec.ts`.
- [x] T5 App shell and styles, README "Run the web app", final checks and report.




