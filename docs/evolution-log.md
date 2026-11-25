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
## P3-mechanics-relativity
- Entry written after the fact (the item's commit b3c5a0c did not update this log); content taken from that commit.
- Nine built-in laws: speed_limit, tsiolkovsky, energy_momentum, relativistic_kinetic_energy, time_dilation, length_contraction, newton_gravity, kepler_third, escape_velocity. Decision D-27 recorded in .kiro/specs/decision-engine/requirements.md.
- Tests: cargo test --workspace 514 pass; web 237 pass.
## P3-thermo-stat-info
- Entry written after the fact (commit 4d96f31 did not update this log); content taken from that commit.
- Built-in laws: landauer (theoretical_bound, Landauer 1961, doi:10.1147/rd.53.0183), stefan_boltzmann (theorem), shannon_hartley (theorem, Shannon 1948, doi:10.1002/j.1538-7305.1948.tb01338.x).
- Tests: cargo test --workspace 524 pass; web 237 pass.
- Deferred laws: Bekenstein bound and Margolus-Levitin bound (optional in the item; not added, reference not verified).
## P3-em-optics
- Built-in laws (data/limits, embedded by Registry::builtin, specs in .kiro/specs/law-<id>/):
  - coulomb: equality, fundamental_law, |q1 q2| / (4 pi eps0 r^2), tolerance 0.5 %, eps0 uncertainty (CODATA 2022) propagated. Refs: Jackson 3rd ed. (Wiley 1999), Griffiths 4th ed. (CUP 2017), NIST CODATA eps0.
  - photon_energy (h f) and photon_energy_wavelength (h c / lambda): equality, fundamental_law. Refs: Griffiths and Schroeter 3rd ed. (CUP 2018), NIST CODATA h.
  - abbe_limit: min bound, theoretical_bound, lambda / (2 NA) for far-field linear imaging; the limitations name STED, PALM/STORM, SIM and near-field microscopy as outside the assumptions. Refs: Abbe 1873, Archiv fuer mikroskopische Anatomie 9(1), 413-468, doi:10.1007/BF02956173 (title, volume and pages checked on api.crossref.org), and Born and Wolf 7th ed. (CUP 1999).
  - luminous_efficacy: max bound, theoretical_bound, Kcd = 683 lm/W (SI definition of the candela, monochromatic 540 THz). Refs: SI Brochure 9th ed. (2019), NIST CODATA Kcd.
  - faraday_emf: equality, fundamental_law, N A |dB| / dt for a fixed loop perpendicular to a uniform field changing at a constant rate. Refs: Griffiths 4th ed., Jackson 3rd ed.
- Deviations: the numerical aperture parameter is named `aperture` because `NA` is reserved (Avogadro constant). abbe_limit has no validity condition `aperture > 0`: a max/min law accepts only temperature-order conditions in the core (UnsupportedCondition), so the condition is stated in the limitations instead. The textbooks are cited from knowledge, not opened; no section numbers given.
- Explorer plots: the explorer is generic over POST /v1/curve, so the new laws are plotted without web changes. API tests check that the Coulomb curve against r and the Abbe curve against wavelength are the engine values of the textbook formulas.
- Deferred laws: maximum luminous efficacy of white light (no verified reference; only the monochromatic definition bound Kcd is included, as the item allows).
- Tests: cargo fmt --check, clippy -D warnings, cargo test --workspace pass (538 Rust tests; new: 11 in crates/limina-dsl/tests/em_optics.rs incl. 3 proptests with fixed seed 42, 2 in crates/limina-api/tests/library.rs). Web: lint, build pass, 237 tests pass (one transient "No test suite found in workbench-logic.spec.ts" on the first run, not reproduced on the rerun).
## P3-quantum-nuclear
- Built-in laws (data/limits, embedded by Registry::builtin, specs in .kiro/specs/law-<id>/):
  - heisenberg: min bound, theorem, dx >= hbar / (2 dp) (Kennard 1927). Refs: Heisenberg, Z. Phys. 43(3-4), 172-198 (1927), doi:10.1007/BF01397280; Kennard, Z. Phys. 44, 326-352 (1927), doi:10.1007/BF01391200; Griffiths and Schroeter 3rd ed. (CUP 2018). Both DOIs checked on api.crossref.org (title, volume, pages). The limitations state that dx and dp are ensemble standard deviations, not measurement errors.
  - de_broglie: equality, fundamental_law, h / p, tolerance 0.5 %. Refs: de Broglie, Annales de Physique 10(3), 22-128 (1925), doi:10.1051/anphys/192510030022 (checked on Crossref); Griffiths and Schroeter.
  - hydrogen_levels: equality, model, binding energy R_inf h c / n^2, tolerance 0.1 % (covers the 0.05 % reduced-mass correction). Non-relativistic, infinite nuclear mass; fine structure and Lamb shift neglected, stated in the limitations. The claim is the magnitude of the binding energy (E_n itself is negative). Refs: Bohr, Phil. Mag. 26(151), 1-25 (1913), doi:10.1080/14786441308634955 (checked on Crossref); Griffiths and Schroeter; NIST CODATA Rydberg constant.
  - radioactive_decay: equality, empirical_law, N0 exp(-t ln2 / T_half), tolerance 1 %. The half-life is a parameter, never invented; Poisson fluctuation of small samples is stated as a limitation. Refs: Rutherford and Soddy, Phil. Mag. 4(21), 370-396 (1902), doi:10.1080/14786440209462856 (checked on Crossref); Krane 3rd ed. (Wiley 1988).
  - nuclear_q: equality, fundamental_law, |m_initial - m_final| c^2, tolerance 0.5 %. Masses are parameters supplied by the user (kg or g), none is looked up. Refs: Einstein, Ann. Phys. 323(13), 639-641 (1905), doi:10.1002/andp.19053231314 (checked on Crossref); Krane.
- Deviations: Q is compared as a magnitude and hydrogen as a binding-energy magnitude, because an equality band around a negative or zero prediction is not meaningful; the sign is stated as not decided. No year time unit exists in the core units table, so decay times are in s, min, h, d; this is stated in the limitations. The textbooks (Griffiths and Schroeter, Krane) are cited from knowledge, not opened; no section numbers given.
- Plots: the explorer is generic over POST /v1/curve. An API test checks the decay curve against t and the Heisenberg hyperbola dx_min against dp as engine values of the formulas. The hatched forbidden region and the decay curve need no web change.
- Deferred laws: none.
- Tests: cargo fmt --check, clippy -D warnings, cargo test --workspace pass (549 Rust tests; new: 11 in crates/limina-dsl/tests/quantum_nuclear.rs incl. 2 proptests with fixed seed 42, 1 in crates/limina-api/tests/library.rs). Web untouched, not rerun.