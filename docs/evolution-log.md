# Evolution log

Items are appended below in order.

## P2-classification

- Added to the DSL definition: optional classification (fundamental_law, theorem, constitutive_relation, theoretical_bound, model, empirical_law), validity_domain, limitations, variables. epistemic_status is unchanged; old JSON loads (tested).
- betz: theoretical_bound; reference Betz, Z. ges. Turbinenwesen 26, 307-309 (1920) (title and pages checked against several citing sources). carnot: theorem; Carnot 1824 and Callen, 2nd ed. (Wiley 1985). cop: theorem; Cengel and Boles, 9th ed. (McGraw-Hill 2019) and Callen. The two textbooks and Carnot 1824 were cited from knowledge, not opened; no section numbers given.
- GET /v1/limits: new optional fields classification, references, validity_domain, limitations. Web: library card shows them, claim badge line shows the classification.
- Decision D-26 recorded in .kiro/specs/decision-engine/requirements.md (the registry test no longer asserts the TODO placeholder; the web test that expected "Reference: to be added" on every card now expects it only where a limit has no reference).
- Tests: cargo fmt, clippy -D warnings and cargo test --workspace pass (new: 3 dsl registry tests, 1 api limits test). Web: lint, build pass, 235 tests pass (one transient suite-load failure on a first run, not reproduced on two reruns).
- Deferred laws: none.

## P2-equality
- DSL: direction `equal` and an optional relative `tolerance` in [0, 1). `EqualityLimit` (crates/limina-dsl/src/equality.rs) compiles one definition into two `DeclarativeLimit`s, upper `F + |F| t` (max) and lower `F - |F| t` (min), run through the unchanged core `decide`. Claim uncertainty, constants' uncertainty and the guard are handled by the engine. `combine_verdicts`: IMPOSSIBLE if either is; COMPATIBLE only if both; INSUFFICIENT_DATA (missing parameters merged) if either is and none is IMPOSSIBLE; else CONDITIONAL (configurations intersected). Table tested exhaustively (16 pairs).
- Equality laws take no assumptions (refused at compile), so CONDITIONAL cannot arise from configurations; the empty-intersection fallback returns the union, never IMPOSSIBLE.
- Validity conditions of an equality law may be any typed comparison (not only temperature orders). A condition that does not hold for the known parameters gives OutsideModelDomain (no verdict); a condition on a missing parameter is left to the engine (INSUFFICIENT_DATA). A condition that cannot be shown to hold counts as failed (no verdict rather than a risky one).
- core is unchanged (no new Direction, Verdict or EngineError variant). `Registry` routes `equal` definitions to `equalities()`; `DeclarativeLimit::compile` refuses `equal` and `tolerance`.
- API: `ApiError::OutsideModelDomain` (422, code OutsideModelDomain); `evaluate_equality` and `response::build_equality` (IMPOSSIBLE wording: "inconsistent with limit X under assumptions none"; bounds = the band); `evaluate_request` dispatches to built-in equality laws; analysis status `outside_model_domain` with the failed condition in the reason and trace. Web: label OUTSIDE MODEL DOMAIN and explanation.
- Not done here (belongs to P3, when the first built-in equality law exists): serving equality laws in GET /v1/limits and in the extraction catalog. No built-in equality law is added in this item; tests use inline definitions (photon energy h f, relativistic kinetic energy with v < c) with expected values written out from the textbook formulas.
- Tests: fmt, clippy -D warnings, cargo test --workspace pass (501 Rust tests; new: 10 in dsl equality incl. 2 proptests with fixed seed 42, 5 in api tests/equality.rs). Web: lint, build pass, 237 tests pass (new: 1).
- Deferred laws: none.