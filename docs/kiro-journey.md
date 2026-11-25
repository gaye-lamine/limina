# Kiro journey

This document is built only from artifacts in this repository: `.kiro/specs`, `.kiro/steering`, `.kiro/hooks`, `docs/journal.md`, the git log, and test files where a section needs them. When a point has no artifact behind it, the text says "no evidence in the repo".

State of the git history when this was written: 11 commits, from `9c5a951 Initial commit` to `08f343f Slice 6: properties and regressions`, all dated 2026-11-15. Only the `decision-engine` spec and five steering files are committed. The other ten specs, the hooks, `ui-design.md`, the crates `limina-api`, `limina-extract` and `limina-dsl`, `web/` and `data/` exist in the working tree but are not committed yet. Commit hashes are therefore available only for the decision-engine work. Everything later is cited by file.

## 1. Spec-driven development

The repo holds 11 specs, each with `requirements.md`, `design.md` and `tasks.md`. Every one is marked "frozen", and every task is ticked except `limit-author` T07.

### decision-engine (crate `limina-core`)
- **What it contains.** Requirements R-01 to R-09: typed quantities, interval arithmetic for uncertainty, verdict rules for Max- and Min-bound limits, missing parameters, determinism, output, the configuration set, and the safety invariant R-09. It also defines properties P-01 to P-06, P-10 and E-01 to E-04, and decisions D-01 to D-23. The design is signatures only. The tasks are T01 to T31 in 8 slices.
- **Version history in git.**
  - `e1e7053 decision-engine spec v5` adds requirements and design.
  - In `51dd476 Add tasks`, the design header moves from "DRAFT v3, awaiting author review" to "v5, frozen". The requirements header moves from "v5, decisions D-18 to D-22 recorded, awaiting author review" to "v5, frozen".
  - `08f343f` appends D-23.
- **Decisions that changed the design.** The decisions section opens with "Recorded from author review of the former open questions". Each decision records the open question it came from (Q-01 to Q-09, O-01, OQ-01 to OQ-07, then OQ-01 to OQ-05 "of design v2").
  - **D-01 and D-15:** the IMPOSSIBLE rule gets a guard `g(x) = max(guard_rel*|x|, guard_abs)`. `design.md` 4.1 records this as a "Deviation from D-01", because a false IMPOSSIBLE is the worst error.
  - **D-02:** Min-bound rules inverted, replacing "the earlier reading".
  - **D-03:** INSUFFICIENT_DATA is decided by comparing a permissive and a restrictive enclosure.
  - **D-04, D-16, D-22:** plausible domains are declared per limit. A parameter with no domain gives INSUFFICIENT_DATA directly, and every missing parameter is tagged `NoDomain` or `HasDomain`.
  - **D-07, D-08, D-17:** empty S is a typed error. S is built from a constraints map, with a hard cap of 100000 configurations.
  - **D-09 superseded by D-12:** the margin becomes the hull of four ratios, so negative claims cannot give an inverted interval.
  - **D-10, D-13, D-19:** C* members are tagged `Certain` or `Depends`. `Depends` lists all missing parameters (a stated simplification).
  - **D-14:** `uom` removed. `51dd476` edits `tech.md` accordingly, and `76ce113 T01: remove uom` removes the dependency.
  - **D-18:** `0 <= guard_rel < 1`, so the quantifier form and the `B_max`/`B_min` form of the rule agree.
  - **D-20:** the guard is global, with no per-limit guard.
  - **D-21:** config validations.
  - **D-23:** a non-finite margin gives `None`. It comes from an open point of slice 5, recorded in `docs/journal.md`.
- **Amendments recorded in `tasks.md`.**
  - api-limit-metadata adds `units_for_dimension`.
  - limit-author adds D-24: the default `guard_abs` becomes 0, and an optional `Limit::guard_abs` floor is added. **Gap:** the tasks.md line points to "requirements.md D-24, design.md section 11", but neither section is present in those files. D-24 is visible only in that tasks.md line, in code comments (`crates/limina-core/src/settings.rs`, `limit.rs`, `engine.rs`), and in `crates/limina-core/tests/guard_scale.rs` and `regressions.rs`.

### limit-betz
- **What it contains.** The Betz bound 16/27, Max, dimensionless, one assumption `reference_area` with three options. The option `shrouded_rotor_area_only` returns a new typed error `NotApplicable`.
- **Decisions.**
  - R-04: an unconstrained claim gives `NotApplicable`, because refusing to decide is never a false IMPOSSIBLE.
  - R-05: mapping `NotApplicable` to INSUFFICIENT_DATA is left to extraction.
  - The reference is left as TODO.
  - `engine.rs` and decision-engine are not changed.

### limit-carnot
- **What it contains.** `1 - Tc/Th`, parameters `Th`, `Tc` as absolute temperatures with no plausible domain, and properties for AC 6 to AC 8.
- **Decisions.**
  - Design 2: "Checked: no existing variant fits", so exactly one variant is added, `InvalidTemperatureOrder`.
  - Missing temperatures rely on D-04 and D-16 of decision-engine.
  - No silent Celsius conversion.

### api-evaluate (crate `limina-api`)
- **What it contains.** R-01 to R-11: `GET /healthz`, `GET /v1/limits`, `POST /v1/evaluate`, fixed wording formulas, a stable error code per `EngineError` variant with no wildcard arm, and pinned versions.
- **Known limits recorded.**
  - `engine_version` is the API crate version.
  - CONDITIONAL cannot occur with Betz and Carnot.
  - The API cannot check that the constraints match the source text.

### api-limit-metadata
- **What it contains.** It amends api-evaluate (additive only). It adds `claim_unit` and `parameters[].units`, plus `units_for_dimension` in `limina-core`.
- **Decisions.**
  - R-03 names one strengthened test.
  - Known limit: `K` is listed but `Quantity::from_unit` rejects it.

### limit-cop
- **What it contains.** Heating `Th/(Th-Tc)` and cooling `Tc/(Th-Tc)` under the assumption `mode`. It is "the first limit whose bound depends on an assumption, so it is the first that can give CONDITIONAL".
- **Decisions.**
  - No new error variant.
  - Exact interval ranges taken at the corners, because the bounds are monotone.
  - One web example.

### web-evaluate (`web/`)
- **What it contains.** An Angular app with an Evaluate page and a Limits section, and the verdict classes of `ui-design.md`.
- **Decisions.**
  - Design: "Angular version decision". Angular 22.2.x needs a newer Node, and the machine runs Node 20.19.2, which "the user will not upgrade". Angular 21.2.x is used, recorded as "a deliberate deviation from current stable".
  - Known limit: `GET /v1/limits` does not expose units, so the UI pre-filled them.

### web-experience
- **What it contains.** It amends web-evaluate: legend, units from metadata, examples, verdict sheet, bound chart, guided recovery, live updates, and "How it works" with the ONTO Limits credit.
- **Decisions.** The "Decisions on open points" section covers three points:
  - A: range bounds.
  - B: no chart without bounds.
  - C: "Metadata mismatch: none after api-limit-metadata".

### extract-claims (crate `limina-extract`)
- **What it contains.** Extraction by a language model behind an `ExtractionProvider` trait, plus security requirements written first:
  - S-01: the secret key.
  - S-02: every value traceable to the text.
  - S-03: prompt injection.
  - R-01 to R-08 cover `/v1/capabilities` and `/v1/analyze`.
- **Decisions.**
  - Constraints are kept only if the option label appears in the text ("makes R-03 checkable without trusting the model").
  - The crate does not depend on `limina-core`.
  - `NotApplicable` maps to "reference area not stated".

### limit-dsl (crate `limina-dsl`)
- **What it contains.** JSON limit definitions, a closed expression grammar with exact SI constants, static dimensional analysis, interval evaluation, a check report, a registry, and differential properties against the Rust limits.
- **Decisions.**
  - Design 3, "Known gap of the core (recorded, not worked around)": the core has only `InvalidTemperatureOrder` for a failed validity condition, so limit-level conditions must compare absolute temperatures.
  - Design 4: naive interval evaluation is conservative and cannot create a false IMPOSSIBLE.

### limit-author
- **What it contains.**
  - `solve_for` for inverse questions.
  - `author_limit`: the language model drafts a DSL definition, with at most 2 repairs.
  - API endpoints `POST /v1/limits/author` and `POST /v1/solve`.
- **Decisions.**
  - The code forces `draft`, the `draft-` id prefix, "to be added" as reference, and never sets `validated_by`.
  - `limina-extract` gains a dependency on `limina-dsl`.
  - New stable 422 codes.
  - Its acceptance criterion 3 (the Landauer case) led to D-24 in decision-engine (see section 5).

## 2. Steering

| File | Inclusion | What it enforced |
|---|---|---|
| `product.md` | always | The verdict comes only from the deterministic engine. Four verdicts. "When in doubt ... never IMPOSSIBLE". Never invent a missing quantity. Wording "inconsistent with limit X under assumptions Y", with no fraud/lie/scam/hoax. Cite ONTO Limits. |
| `structure.md` | always | Crate layout. `.kiro/` must be committed. One limit = one file + one spec + golden tests. No `unwrap()` in library code. `thiserror`. Documented public items. |
| `tech.md` | always | Rust workspace with an in-house runtime `Dimension`; `uom` rejected, an edit made in `51dd476`. `proptest`. axum + tokio. LLM only through `ExtractionProvider`. No secrets in the repo. Pinned versions. The fmt, clippy and test commands. |
| `testing.md` | fileMatch `crates/**/tests/**/*.rs` | Property tests for every decision rule. Fixed seeds in CI. Keep failing cases as regression tests. "Never classify a value below the bound as IMPOSSIBLE". Readable shrunk counterexamples. |
| `physics-rules.md` | fileMatch `crates/limina-core/src/limits/**/*.rs` | Declare id, direction, dimension, assumptions, validity, references and epistemic status. Never invent a constant, formula or reference ("STOP and ask"). Kelvin inside the engine. Golden and property tests. |
| `ui-design.md` (not committed) | fileMatch `web/**` | Black, white and grey only. Verdicts never encoded by color. Every string in one file. WCAG 2.2 AA. The SVG schematic style. Wording shown as received. |

Traces of these rules in the specs:
- `testing.md` and `product.md`: the "never IMPOSSIBLE at or below the bound" property in every limit spec.
- `physics-rules.md`: "Reference: TODO ... No reference is invented here" in betz, carnot and cop.
- `tech.md`: the `ExtractionProvider` trait in extract-claims.
- `ui-design.md`: web-evaluate R-06.
- Gaps: `tech.md` lists SQLite/sqlx, wasm-bindgen, D3, KaTeX, fast-check and Playwright, and `structure.md` lists `limina-wasm`, `limina-mcp` and `docs/methodology.md`. None of these is present in the repo, and `web/package.json` has none of the listed web libraries.

## 3. Hooks

| Hook | Trigger | What it does | Evidence that it ran |
|---|---|---|---|
| `core-tests-on-save` | `PostFileSave`, matcher `crates/limina-core/(src\|tests)/*.rs` | Runs `cargo fmt --all --check`, clippy `-D warnings` and `cargo test --workspace`, then prints PASS/FAIL. Never modifies files. | No evidence in the repo. The hook writes no log or file. |
| `submission-guard` | `SessionStart` | Read-only checks, each reported PASS/FAIL: `.gitignore` does not exclude `.kiro`, LICENSE exists, README has an installation section, `.kiro/steering`, `specs` and `hooks` are tracked by git, Cargo.lock exists. | No evidence in the repo. The hook writes no log or file. |

Both hook files are staged in git but not committed. Check (d) of `submission-guard` would currently find tracked files in all three directories: the hooks are staged and the decision-engine spec and steering files are committed.

## 4. Property-based testing

Property tests use `proptest` 1.12.0 with a fixed seed (`SEED = 0x4C49_4D49_4E41`, "LIMINA", in `crates/limina-core/tests/common/scenario.rs`). They run 256 cases locally and 1024 when `CI` is set. The oracle is independent: it recomputes the guard with its own formula.

Properties found in the test files:
- **decision-engine** (`limina-core/tests`):
  - P-01, P-10: `sets.rs`.
  - P-02, P-03 (a) and (b), E-01, E-02, E-04: `rules_props.rs`.
  - P-02, P-03 (b), E-02, E-04 with missing parameters: `missing_props.rs`.
  - E-03: `engine_props.rs`.
  - P-04: `units_equivalence.rs`.
  - P-05, P-06: `units.rs`.
  - P-07 to P-09 are intentionally absent (D-05).
- **Limits:**
  - `limit_betz.rs`: never IMPOSSIBLE at or below 16/27.
  - `limit_carnot.rs`: bound strictly below 1; increasing in Th; decreasing in Tc; never IMPOSSIBLE at or below the bound.
  - `limit_cop.rs`: heating = cooling + 1; both bounds decrease as the gap grows; never IMPOSSIBLE at or below the bound.
- **D-24 scale property** (`guard_scale.rs`): for bounds from 1e-30 to 1e30, a claim at or below the bound is never IMPOSSIBLE, and a claim beyond `b*(1±1e-6)` is IMPOSSIBLE (Max and Min).
- **API** (`limina-api/tests/evaluate.rs`): Betz and Carnot claims below the bound are never IMPOSSIBLE through HTTP.
- **DSL** (`limina-dsl/tests/differential.rs`): the DSL and the Rust limits give the same bound, error, verdict and C* for betz, carnot and cop.
- **solve_for** (`limina-dsl/tests/solve.rs`): closed-form oracles for Carnot, COP heating and COP cooling, and a non-monotone limit refused.

Failures:

| Label | Where | Property | Shrunk counterexample | Evidence |
|---|---|---|---|---|
| **Injected** (mutation, slice 5) | `rules_max`: `lo > b + g(b)` replaced by `lo > b` | P-03 (a) | `Max-bound \| B(0)=0.0 \| claim v=1e-12 u=0.0 k=1.0 -> lo=1e-12 hi=1e-12 \| guard Default (rel=1e-9, abs=1e-12)`. The mutated engine says IMPOSSIBLE, the oracle CONDITIONAL. | `docs/journal.md` "Expérience de mutation (tranche 5)". Kept as `regression_mutation_p03a_...` in `tests/regressions.rs`. |
| **Injected** (same mutation) | same | P-02 | `Max-bound \| B(0)=0.0 \| claim v=5.498937935469428e-13 u=0.0 k=1.0 \| guard Default` | `docs/journal.md`. Kept as `regression_mutation_p02_...` in `tests/regressions.rs`. E-01 stayed green, as expected (no guard). |
| **Real**, during development | `limit_carnot.rs` | Not named in the file; the parameter names match `bound_increases_with_th` | `(th, tc) = (1.0002965067023164, 1.0), factor = 1.001` | `crates/limina-core/tests/limit_carnot.proptest-regressions` (not committed). Cause and fix: no evidence in the repo. |
| **Real**, during development | `limit_carnot.rs` | Not named; one parameter pair only, matching `bound_is_strictly_below_one` | `(th, tc) = (2.0, 1.0)` | same file |
| **Real**, during development | `limit_carnot.rs` | Not named; the parameters match `claim_at_or_below_bound_is_never_impossible` | `(th, tc) = (2.0, 1.0), t = 0.9610293284206198, pick = 0` | same file |
| **Real**, during development | `limit_carnot.rs` | Not named; the parameters match `bound_decreases_with_tc` | `(th, tc) = (2.0, 1.0), t = 0.9034339647386033` | same file |

The journal says "aucun bogue réel trouvé" for slice 5 and "aucune défaillance" for slice 6 (149 tests, 18 properties at 1024 cases). The four Carnot cases are not copied into `tests/regressions.rs` as named tests, as `testing.md` asks, and their regression file is untracked. The mutation cases were found with the old default absolute guard of 1e-12. Since D-24 the two regression tests keep that floor as an explicit guard mode (`GuardMode::AbsFloor`).

## 5. Moments where a spec or an agent stopped and flagged a problem

| Moment | What was flagged | Evidence |
|---|---|---|
| decision-engine review rounds | Open questions Q-01 to Q-09, O-01, OQ-01 to OQ-07 and five more OQ in design v2 were raised in the drafts and settled by the author as D-01 to D-22. | `requirements.md` decisions ("Recorded from author review of the former open questions"); `design.md` section 10. The header changes are in `51dd476`. |
| Guard on IMPOSSIBLE | The strict rule of D-01 could give a false IMPOSSIBLE through rounding. | `design.md` 4.1 "Deviation from D-01"; D-15. |
| Coverage gap R-07.5 | The public doc strings were not tested for banned words. | `docs/journal.md` slice 3: "trou de couverture R-07.5 repéré par Kiro (doc strings)". Follow-up task T31 in `decision-engine/tasks.md`. |
| Non-point bound from point parameters | A limit that breaks the `Limit::bound` contract. | `docs/journal.md`, slice 5 decisions: `InsufficientData` with an empty list, "la spec n'est pas rouverte". |
| Infinite margins | An open point in slice 5. | D-23 in `requirements.md`, added in `08f343f`. |
| Betz with an unstated reference area | Shrouded claims referenced to the rotor area alone can exceed 16/27. | `limit-betz/requirements.md` R-04, R-06. Who raised it: no evidence in the repo. |
| Carnot error variant | No existing `EngineError` fit `Th <= Tc`. | `limit-carnot/design.md` 2, "Checked: no existing variant fits". |
| CONDITIONAL never reachable with Betz and Carnot | — | `api-evaluate/requirements.md` "Known limits". `limit-cop` is introduced as "the first that can give CONDITIONAL". |
| Node 20 against Angular 22 | The current Angular cannot run on this machine. | `web-evaluate/design.md` "Angular version decision". |
| Units missing from the API metadata | The UI could not offer units. | `web-evaluate/requirements.md` "Known limits". `api-limit-metadata` spec. `web-experience` decision C. |
| `K` rejected by `Quantity::from_unit` | — | `api-limit-metadata/requirements.md` "Known limit". |
| Core has no generic validity error | — | `limit-dsl/design.md` section 3 "Known gap of the core (recorded, not worked around)". |
| Absolute guard blocks IMPOSSIBLE at small scales | With `guard_abs = 1e-12` J, a Landauer bound near 2.9e-21 J could never give IMPOSSIBLE, so limit-author AC 3 could not hold without changing the core. | `decision-engine/tasks.md` last line (D-24). Doc comments in `crates/limina-core/src/settings.rs` and `tolerance.rs`. `crates/limina-core/tests/guard_scale.rs`. The full D-24 text is missing from requirements.md and design.md (gap, section 1). |

## 6. What did not work

- **Workflow overhead.** No evidence in the repo. No workflow file or run artifact is committed or present (`.agents/` does not exist, and `.kiro/` has no workflows directory).
- **The requirements.md rewrite from memory.** No evidence in the repo. In git, `decision-engine/requirements.md` appears once, already at v5 (`e1e7053`), then gets only a header change (`51dd476`) and the D-23 line (`08f343f`). Nothing in the files or the journal mentions a rewrite.
- **Other gaps visible in the repo:**
  - Most of the work after slice 6 is not committed.
  - The `docs/journal.md` template entry ("YYYY-MM-DD") was never filled. The journal covers only slices 1 to 6. Its "écarts consignés" (deviations recorded) are counted, but the deviations themselves are not written down.
  - D-24 is referenced but its sections are missing (section 1).
  - Real Carnot shrunk cases were not promoted to named regression tests (section 4).
  - Several steering items describe tools or crates that do not exist yet (section 2).

## Hackathon write-up questions

| Question | Evidence found |
|---|---|
| IDE / CLI / Web / Crew | No evidence in the repo of which client was used. The hook triggers `PostFileSave` and `SessionStart` are recorded in `.kiro/hooks/*.json`. |
| Custom agents | Not used (no agent definitions in `.kiro/`). |
| Hooks | Two hook files: `core-tests-on-save` (PostFileSave) and `submission-guard` (SessionStart). No evidence in the repo that they ran. |
| Spec-driven vs vibe coding | Spec-driven: 11 specs, each with requirements, design and tasks; D-01 to D-23 (plus D-24, referenced) with their origin questions; tasks ticked per slice; frozen statuses. Vibe coding: no evidence in the repo. |
| Steering | 6 files: 3 always-included (product, structure, tech), 3 by file match (testing, physics-rules, ui-design). |
| MCP | Not used. No `.kiro/settings/mcp.json`. `limina-mcp` is planned in `structure.md` but absent. |
| Property-based testing | Used: proptest with a fixed seed, 256/1024 cases, an independent oracle, one injected mutation with two shrunk counterexamples, and four real shrunk Carnot cases in a regression file. |
| Powers | Not used (no evidence in the repo). |
