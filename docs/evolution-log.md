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
- Tests: cargo fmt --check, clippy -D warnings, cargo test --workspace pass (550 Rust tests; new: 11 in crates/limina-dsl/tests/quantum_nuclear.rs incl. 2 proptests with fixed seed 42, 1 in crates/limina-api/tests/library.rs). Web untouched, not rerun.

## P3-astro-cosmo
- Constant added to the core table (CODATA 2022, NIST allascii.txt, line checked in data/constants/codata-2022-allascii.txt): sigma_T, Thomson cross section, 6.6524587051e-29 m^2, standard uncertainty 0.0000000062e-29 m^2. A test checks it against 8 pi r_e^2 / 3 with the CODATA classical electron radius.
- Built-in laws (data/limits, embedded by Registry::builtin, specs in .kiro/specs/law-<id>/):
  - eddington_luminosity: max bound, theoretical_bound, L <= 4 pi G M m_p c / sigma_T. Assumption accretion_geometry with two options: spherical_steady_ionized_hydrogen (the formula) and non_spherical_or_other_composition (no formula, like Betz's third option). Without the assumption, or with the second option, the engine returns NotApplicable (needs input), never a verdict: super-Eddington luminosities are physically possible in other geometries, so a verdict without the assumption could be a false IMPOSSIBLE. Refs: Rybicki and Lightman, Radiative Processes in Astrophysics (Wiley 1979); Frank, King and Raine, Accretion Power in Astrophysics, 3rd ed. (CUP 2002); CODATA sigma_e.
  - schwarzschild_radius: equality, model, 2 G M / c^2, tolerance 0.5 %. Ref: Schwarzschild 1916, English translation arXiv:physics/9905030, doi:10.48550/arXiv.physics/9905030 (title and author checked on api.datacite.org); Misner, Thorne and Wheeler, Gravitation (Freeman 1973).
  - friedmann_flat: equality, model, H = sqrt(8 pi G rho / 3), tolerance 0.5 %, validity rho > 0. The k = 0 case is expressed cleanly by the DSL, so it is not deferred. The claim unit is Hz because "1/s" is not parsed by the units table. Refs: Friedmann, Z. Phys. 10, 377-386 (1922), doi:10.1007/BF01332580 (title, volume, pages checked on api.crossref.org); Ryden, Introduction to Cosmology, 2nd ed. (CUP), cited from knowledge without year.
- Deviations: at the Eddington bound itself the uncertainty of G, m_p and sigma_T straddles the claim, so the engine returns INSUFFICIENT_DATA with no missing parameter instead of COMPATIBLE (existing D-03 behaviour); the test asserts "not IMPOSSIBLE" there. No solar-mass unit and no km/s/Mpc unit exist in the core units table, stated in the limitations. The textbooks are cited from knowledge, not opened; no section numbers given.
- Deferred laws: none.
- Tests: cargo fmt --check, clippy -D warnings, cargo test --workspace pass (562 Rust tests; new: 12 in crates/limina-dsl/tests/astro_cosmo.rs incl. 2 proptests with fixed seed 42). Web untouched, not rerun.

## P4-identification
- The extraction prompt already listed every built-in law (id, title, formula, claim unit, parameters with units, assumptions with options) because the catalog is built from GET /v1/limits. The law-naming prompt now lists the library too.
- New law-naming step with the library: `identify_law_in_library` (limina-extract) offers the built-in laws to the model, which answers `built-in: <id>`, one sentence, or "no law identified". The id is checked against the catalog; an unknown id is an error (no law is guessed, status needs_input). `identify_law` is unchanged.
- Pipeline: a claim with limit_id null goes to the naming step. Built-in id: parameters read by the second pass with the catalog units, evaluated by the deterministic engine (equality laws included), no authoring call, trace step "law": "built-in limit X identified by law naming; no authoring". A sentence goes to two-model authoring as before. No law: NEEDS INPUT as before. On the naming path no assumption is passed to the engine (none is stated), so laws with assumptions answer needs_input or CONDITIONAL, never a guessed verdict. Duplicate merging is unchanged.
- Web: the pre-filled text is now six paragraphs (relativity, quantum, optics, thermodynamics, computing, battery), labelled illustrative.
- Tests: cargo fmt --check, clippy -D warnings, cargo test --workspace pass (565 Rust tests; new: 2 mock pipeline tests for the built-in path and an unknown id, 1 extract test for the answer parser, 1 ignored live test). Web: lint, build pass, 237 tests pass. The first web run showed again the transient "No test suite found in workbench-logic.spec.ts" (a fixture helper file with no `it` at the top level); the rerun was green.
- Live smoke (model from .env, one request per paragraph, raw output of the last of two runs):
    TEXT The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.
    HTTP 200 OK
    claim "m/s" limit "speed_limit" status "evaluated" verdict "IMPOSSIBLE" source "built_in"
      ["done"] "extraction": "read m/s: 3.6e8 m/s, evidence \"The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.\""
      ["done"] "limit": "built-in limit speed_limit"
      ["done"] "parameters": "the limit needs no parameter"
      ["done"] "evaluation": "engine verdict IMPOSSIBLE"
    TEXT The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.
    HTTP 200 OK
    claim "h/(2*dp)" limit "heisenberg" status "evaluated" verdict "IMPOSSIBLE" source "built_in"
      ["done"] "extraction": "read h/(2*dp): 1e-12 m, evidence \"The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.\""
      ["done"] "limit": "built-in limit heisenberg"
      ["done"] "parameters": "found dp = 1.0000000000000001e-24 kg*m/s"
      ["done"] "evaluation": "engine verdict IMPOSSIBLE"
    TEXT The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.
    HTTP 502 Bad Gateway
      body: {"error":{"code":"ExtractionFailed","message":"extraction failed: the answer does not match the schema"}}
    TEXT The Aurora heat pump delivers heat with a COP of 14.5, hot reservoir at 300 K, cold reservoir at 280 K.
    HTTP 502 Bad Gateway
      body: {"error":{"code":"ExtractionFailed","message":"extraction failed: the answer does not match the schema"}}
    TEXT The Quanta logic chip erases one bit at 300 K while dissipating 1e-22 J.
    HTTP 502 Bad Gateway
      body: {"error":{"code":"ExtractionFailed","message":"extraction failed: the answer holds no JSON object"}}
    TEXT The Volta cell stores 5 kWh of energy in a mass of 1 kg.
    HTTP 200 OK
    claim "energy" limit null status "needs_input" verdict null source "no_law_identified"
      ["done"] "extraction": "read energy: 5 kWh, evidence \"The Volta cell stores 5 kWh of energy in a mass of 1 kg.\"; flags: limit not recognized"
      ["failed"] "limit": "no built-in limit matches this claim"
      ["failed"] "law": "no law identified"
    claim "mass" limit null status "needs_input" verdict null source "no_law_identified"
      ["done"] "extraction": "read mass: 1 kg, evidence \"The Volta cell stores 5 kWh of energy in a mass of 1 kg.\"; flags: limit not recognized"
      ["failed"] "limit": "no built-in limit matches this claim"
      ["failed"] "law": "no law identified"
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 6.45s
- Reading of the live runs: the extraction step maps the claims to built-in laws directly, so the naming path with the library was not exercised live (it is covered by the mock tests). The extraction model is unstable on this text: the first run failed on 2 paragraphs (Lumen, Quanta) and read the COP claim with unit "%" (verdict COMPATIBLE on that misreading); the second run failed on 3 paragraphs (Lumen, Aurora, Quanta) with HTTP 502 (answer not matching the schema, or no JSON object). Failures are recorded as is, not retried by the test. Where a verdict was produced it is the engine's (speed_limit and heisenberg IMPOSSIBLE). The Volta battery claim stays NO LAW IDENTIFIED, with no verdict.
- Deferred laws: none.

## P5-graphs: scientific graphs computed by the engine

- Engine side (limina-dsl): bound_curve_enclosed (fixed parameters as SI enclosures, so the band carries parameter and constant uncertainties), zones/validity_zones, bound_surface (grid capped at 50 x 50), EqualityLimit::outside_domain. limina-core is unchanged.
- API: POST /v1/curve gains optional fields only: kind (bound or equality), lower (lower edge of an equality tolerance band), band (request field uncertainties, absolute, in the unit of the value), validity_zones. New POST /v1/surface. Equality laws now also have a default range from their definition.
- Web: graph-geometry.ts (curve with forbidden region, uncertainty band, validity zones; equality prediction with tolerance band; heat map with claim contour at cell resolution; log comparison scale for parameterless laws). The explorer picks the view from the law: curve, a Curve/Map switch from two parameters, comparison scale with no parameter, and a parameter uncertainty selector (0, 1, 5, 10 %). A log scale that cannot be drawn (non-positive value) says so.
- Not done: field plots (Coulomb field lines, gravity potential). No engine endpoint computes a field from a law, so none is drawn; this is the stated limitation, no figure is invented.
- Tests: 8 new API tests (tests/graphs.rs: band, validity zones of a bound law and of an equality law, equality tolerance band against the textbook formula, surface against 1 - Tc/Th, grid cap, errors), 13 new web geometry tests (graph-geometry.spec.ts).
- Layout check, headless Edge at 1920x1080 with the live API on the illustrative text, comparison scale view (speed_limit claim): window height 1080, document height 1080, body height 1080. Only scrolling element: the input textarea (pre-existing, user text). The map view was not measured: the claim open in the panel had no parameter, so the Map switch was not shown. The curve view was not measured either for the same reason; its svg keeps the same max-height as before.


## P6-vectors
- Assessment: a real vector type (3 components, cross product, interval evaluation per component) would need new expression types and dimension checks in limina-dsl and a claim form for vectors. Not done: it would be a design change to the DSL and the claim model, and the one law in scope does not need it.
- Smallest safe version, implemented: magnitude-only evaluation with the field and speed magnitudes as explicit parameters. Law lorentz_force (max bound, theorem, data/limits/lorentz_force.json, spec .kiro/specs/law-lorentz_force/): |F| <= |q| (|E| + |v| |B|), from the triangle inequality and |v x B| <= |v||B|. Refs: Jackson, Classical Electrodynamics, 3rd ed., Wiley (1999); Griffiths, Introduction to Electrodynamics, 4th ed., CUP (2017), cited from knowledge, not opened.
- Limits stated in the law: only an excess over the bound is excluded; a force below it is not confirmed (the directions are not modelled). Directions of E, B and v stay out of scope.
- Behaviour noted: a claim exactly at the bound gives CONDITIONAL (interval rounding), never IMPOSSIBLE; the test asserts not-IMPOSSIBLE there.
- Open decision for the author: whether to add true vector quantities (components, cross product) to the DSL, which would allow a tight equality F = q (E + v x B) instead of a bound.
- Tests: cargo fmt --check, clippy -D warnings, cargo test --workspace pass (new: 4 in crates/limina-dsl/tests/lorentz.rs incl. 1 proptest with fixed ChaCha seed, 256 cases, 1024 with CI). Registry and API list tests updated for the 13th max/min law. Web untouched, not rerun.
- Deferred laws: none.
## P7-model-fallback
- limina-extract: `FallbackProvider` (implements `ExtractionProvider`) and `FallbackVision` (implements `VisionProvider`) over an ordered list of models (crates/limina-extract/src/fallback.rs). The next model is tried on a provider error (timeout, transport, HTTP 404, 429, 5xx) or an unusable answer (empty content, finish_reason length, no balanced JSON object, a JSON object that does not parse). Any other HTTP status (401, 403, 400) stops the chain with its typed error, since the next model would get the same answer. Every model has its own timeout (`LIMINA_LLM_TIMEOUT_SECS`, default 15 s; vision keeps 60 s). A model that failed on the provider side is skipped for 60 s (circuit breaker). An unusable answer does not open the breaker. When every model of a chain is cooling down, they are all tried anyway.
- Trait growth only: `ExtractionProvider::complete_routed` and `VisionProvider::transcribe_routed` with default implementations; new `Route` and `Routed` types; new `ProviderError::Unusable` variant (limina-extract, not limina-core; no other crate matches on ProviderError exhaustively).
- Chains of the deployed server (AppState::from_config): extraction checks the extraction schema too (`ReplyKind::Checked(extraction_problem)`), so a JSON object that does not fit the schema also falls through. Deviation from the item text: the item lists "a JSON object that does not parse"; the extraction chain goes one step further because the live failures before this change were schema mismatches (HTTP 502 "the answer does not match the schema"). Parameters and authoring model A use the same models with the plain JSON check, law naming with a text check (a sentence is a valid answer). Authoring model B has its own chain.
- Configuration, from the environment only: `LIMINA_LLM_MODELS` (default lightning, bolt, laguna, ultra), `LIMINA_LLM_MODELS_B` (default laguna, ultra), `LIMINA_LLM_VISION_MODELS` (default super-vl, nano-omni). `LIMINA_LLM_MODEL`, `LIMINA_LLM_MODEL_B` and `LIMINA_LLM_VISION_MODEL` still work and are put first in their chain. Blank items and duplicates are dropped.
- Trace: the extraction, law naming and parameters steps end with "model M" or "model M; skipped: X (reason), ...". Authoring steps are named after the model that wrote the definition and list the skipped models. GET /v1/pipeline gains an optional `chains` field {extraction, authoring_b, vision}; `models` is unchanged.
- Two-model comparison: when chain B falls back to the model that wrote A's definition, the law is `draft_disputed` (one author only, not compared), with the trace detail "both definitions were written by the same model M (chain B fell back to it); one author only, not compared". It is never `draft_agreed`.
- Tests: cargo fmt --check, clippy -D warnings, cargo test --workspace pass (590 Rust tests; new: 12 in crates/limina-extract/tests/fallback.rs covering first fails then second answers, 404/429/5xx fall through and 401 does not, all fail gives the typed error, each kind of unusable answer, schema mismatch on the checked chain, circuit breaker skip, close after cool-down, all cooling down, chain B falling back to A's model never agrees, two distinct models still agree; 1 config test; 3 in crates/limina-api/tests/pipeline.rs for the model named in each trace step, same-model disputed through the API, and the chains in GET /v1/pipeline). The extract-claims tests and the key-never-printed tests (limina-extract/tests/security.rs, limina-api/tests/security.rs, limina-cli/tests/check.rs) pass unchanged. Web untouched (the new `chains` field is optional), not rerun.
- Live smoke on the six-paragraph illustrative text (pipeline_live::the_multi_discipline_text_is_identified_from_the_library_first, chains from the defaults, one request per paragraph). Paragraphs that failed (HTTP 502, no claim): before this change 3 of 6 on the last P4 run (Lumen, Aurora, Quanta; 2 of 6 on the first P4 run), after it 0 of 6 on the run below. A first run after the change, with only the JSON check on the extraction chain, still failed 2 of 6 (Lumen, Aurora, schema mismatches), which led to the schema check above.
- Reading of the run below: every verdict comes from the engine. speed_limit and landauer IMPOSSIBLE; the COP claim is now read as 14.5 dimensionless and gets CONDITIONAL (no heating/cooling constraint stated). Model misreadings remain and are recorded as is: the Q-Scope paragraph was read as a claim of "1 dimensionless" with no evidence and stopped at needs_input; the Lumen 100 nm resolution was flagged "unit not understood" (needs_input), and the 500 nm wavelength was mapped by law naming to abbe_limit and compared as if it were the resolved size (COMPATIBLE, a meaningless comparison but not a false IMPOSSIBLE). The battery claim stays NO LAW IDENTIFIED with no verdict. The models that answered vary per paragraph (lightning, laguna, ultra); the skipped models and reasons are in the trace.
- Raw output:
    TEXT The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.
    HTTP 200 OK
    claim "m/s" limit "speed_limit" status "evaluated" verdict "IMPOSSIBLE" source "built_in"
      ["done"] "extraction": "read m/s: 3.6e8 m/s, evidence \"The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.\"; model nvidia/nemotron-3.5-lightning-30b-a3b"
      ["done"] "limit": "built-in limit speed_limit"
      ["done"] "parameters": "the limit needs no parameter"
      ["done"] "evaluation": "engine verdict IMPOSSIBLE"
    TEXT The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.
    HTTP 200 OK
    claim "dimensionless" limit "heisenberg" status "needs_input" verdict null source "built_in"
      ["done"] "extraction": "read dimensionless: 1 dimensionless, no evidence found in the text; flags: not traceable to the text, unit not understood; model poolside/laguna-xs-2.1; skipped: nvidia/nemotron-3.5-lightning-30b-a3b (the answer does not match the schema), nvidia/nemotron-3.5-bolt-e4b (no JSON object in the answer)"
      ["done"] "limit": "built-in limit heisenberg"
      ["failed"] "parameters": "missing dp (not stated in the text)"
      ["failed"] "evaluation": "not evaluated: unit not understood"
    TEXT The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.
    HTTP 200 OK
    claim "resolution" limit "abbe_limit" status "needs_input" verdict null source "built_in"
      ["done"] "extraction": "read resolution: 100 nm, evidence \"The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.\"; flags: unit not understood; model nvidia/nemotron-3-ultra-550b-a55b; skipped: nvidia/nemotron-3.5-lightning-30b-a3b (the answer does not match the schema), nvidia/nemotron-3.5-bolt-e4b (no JSON object in the answer), poolside/laguna-xs-2.1 (the answer does not match the schema)"
      ["done"] "limit": "built-in limit abbe_limit"
      ["done"] "parameters": "found wavelength = 500 nm, aperture = 1.4 dimensionless"
      ["failed"] "evaluation": "not evaluated: unit not understood"
    claim "wavelength" limit "abbe_limit" status "evaluated" verdict "COMPATIBLE" source "built_in"
      ["done"] "extraction": "read wavelength: 500 nm, evidence \"The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.\"; flags: limit not recognized; model nvidia/nemotron-3-ultra-550b-a55b; skipped: nvidia/nemotron-3.5-lightning-30b-a3b (the answer does not match the schema), nvidia/nemotron-3.5-bolt-e4b (no JSON object in the answer), poolside/laguna-xs-2.1 (the answer does not match the schema)"
      ["failed"] "limit": "no built-in limit matches this claim"
      ["done"] "law": "built-in limit abbe_limit identified by law naming; no authoring (model nvidia/nemotron-3.5-lightning-30b-a3b)"
      ["done"] "parameters": "found wavelength = 500 nm, aperture = 1.4 dimensionless; model nvidia/nemotron-3.5-lightning-30b-a3b"
      ["done"] "evaluation": "engine verdict COMPATIBLE"
    claim "numerical aperture" limit null status "needs_input" verdict null source "not_authored"
      ["done"] "extraction": "read numerical aperture: 1.4 dimensionless, evidence \"The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.\"; flags: limit not recognized; model nvidia/nemotron-3-ultra-550b-a55b; skipped: nvidia/nemotron-3.5-lightning-30b-a3b (the answer does not match the schema), nvidia/nemotron-3.5-bolt-e4b (no JSON object in the answer), poolside/laguna-xs-2.1 (the answer does not match the schema)"
      ["failed"] "limit": "no built-in limit matches this claim"
      ["done"] "law": "abbe_limit: wavelength/(2*aperture) (model nvidia/nemotron-3.5-lightning-30b-a3b)"
      ["failed"] "authoring by nvidia/nemotron-3.5-lightning-30b-a3b": "no valid definition after 3 attempts; last error: the definition does not match the schema: unknown field `test`, expected one of `name`, `dimension`, `kind`, `units`, `required`, `plausible`, `test_range`"
      ["failed"] "authoring by poolside/laguna-xs-2.1": "no valid definition after 3 attempts; last error: the definition does not match the schema: unknown variant `dimension`, expected `absolute_temperature` or `temperature_difference`"
      ["skipped"] "comparison": "not_authored: neither model wrote a valid definition"
      ["skipped"] "parameters": "no agreed law"
      ["skipped"] "evaluation": "not evaluated: no valid definition of the law could be written"
    TEXT The Aurora heat pump delivers heat with a COP of 14.5, hot reservoir at 300 K, cold reservoir at 280 K.
    HTTP 200 OK
    claim "COP" limit "cop" status "evaluated" verdict "CONDITIONAL" source "built_in"
      ["done"] "extraction": "read COP: 14.5 dimensionless, evidence \"The Aurora heat pump delivers heat with a COP of 14.5\"; model poolside/laguna-xs-2.1; skipped: nvidia/nemotron-3.5-lightning-30b-a3b (the answer does not match the schema), nvidia/nemotron-3.5-bolt-e4b (the JSON object does not parse)"
      ["done"] "limit": "built-in limit cop"
      ["done"] "parameters": "found Th = 300 K, Tc = 280 K"
      ["done"] "evaluation": "engine verdict CONDITIONAL"
    TEXT The Quanta logic chip erases one bit at 300 K while dissipating 1e-22 J.
    HTTP 200 OK
    claim "energy" limit "landauer" status "evaluated" verdict "IMPOSSIBLE" source "built_in"
      ["done"] "extraction": "read energy: 1e-22 J, evidence \"dissipating 1e-22 J\"; model poolside/laguna-xs-2.1; skipped: nvidia/nemotron-3.5-lightning-30b-a3b (the JSON object does not parse), nvidia/nemotron-3.5-bolt-e4b (no JSON object in the answer)"
      ["done"] "limit": "built-in limit landauer"
      ["done"] "parameters": "found T = 300 K"
      ["done"] "evaluation": "engine verdict IMPOSSIBLE"
    TEXT The Volta cell stores 5 kWh of energy in a mass of 1 kg.
    HTTP 200 OK
    claim "energy" limit null status "needs_input" verdict null source "no_law_identified"
      ["done"] "extraction": "read energy: 5 kWh, evidence \"The Volta cell stores 5 kWh of energy in a mass of 1 kg.\"; flags: limit not recognized; model nvidia/nemotron-3.5-lightning-30b-a3b"
      ["failed"] "limit": "no built-in limit matches this claim"
      ["failed"] "law": "no law identified (model nvidia/nemotron-3.5-lightning-30b-a3b)"
    claim "mass" limit null status "needs_input" verdict null source "no_law_identified"
      ["done"] "extraction": "read mass: 1 kg, evidence \"The Volta cell stores 5 kWh of energy in a mass of 1 kg.\"; flags: limit not recognized; model nvidia/nemotron-3.5-lightning-30b-a3b"
      ["failed"] "limit": "no built-in limit matches this claim"
      ["failed"] "law": "no law identified (model nvidia/nemotron-3.5-lightning-30b-a3b)"
    test the_multi_discipline_text_is_identified_from_the_library_first ... ok
- Deferred laws: none.