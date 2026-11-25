# Evolution log

Items are appended below in order.

## P2-classification

- Added to the DSL definition: optional classification (fundamental_law, theorem, constitutive_relation, theoretical_bound, model, empirical_law), validity_domain, limitations, variables. epistemic_status is unchanged; old JSON loads (tested).
- betz: theoretical_bound; reference Betz, Z. ges. Turbinenwesen 26, 307-309 (1920) (title and pages checked against several citing sources). carnot: theorem; Carnot 1824 and Callen, 2nd ed. (Wiley 1985). cop: theorem; Cengel and Boles, 9th ed. (McGraw-Hill 2019) and Callen. The two textbooks and Carnot 1824 were cited from knowledge, not opened; no section numbers given.
- GET /v1/limits: new optional fields classification, references, validity_domain, limitations. Web: library card shows them, claim badge line shows the classification.
- Decision D-26 recorded in .kiro/specs/decision-engine/requirements.md (the registry test no longer asserts the TODO placeholder; the web test that expected "Reference: to be added" on every card now expects it only where a limit has no reference).
- Tests: cargo fmt, clippy -D warnings and cargo test --workspace pass (new: 3 dsl registry tests, 1 api limits test). Web: lint, build pass, 235 tests pass (one transient suite-load failure on a first run, not reproduced on two reruns).
- Deferred laws: none.
