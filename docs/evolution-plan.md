# Limina: evolution towards fundamental physics as a whole

Status: plan v1. Scope decided by the author: every discipline of fundamental physics is in scope; no law is ever invented to produce a verdict; the verdict stays with the deterministic engine.

## 1. Audit of the current architecture (commit bac1a61)

| Area | What exists | What blocks general physics |
|---|---|---|
| `limina-core` units | 13 exact symbols (W, kW, MW, J, kWh, K, m, s, kg, mol, A, cd, dimensionless); no prefixes, no compound units | m/s, N, Pa, V, Ω, T, Hz, eV, W/m², kWh/kg, bit/s cannot be read at all |
| `limina-core` dimensions | 7 SI exponents, runtime `Dimension`; absolute temperature and temperature difference as their own types | sound; reused as is |
| `limina-core` engine | `decide` on one bounded quantity, Max or Min, four verdicts, guard relative only (D-24), uncertainty `v ± k·u`, missing parameters with plausible domains | an equality law (Coulomb, de Broglie) has no direction; vectors are not represented |
| `limina-core` limits | Betz, Carnot, COP in Rust | three laws, all on energy conversion |
| constants | none in the core; the DSL has 7 (kB, h, c, e, NA, pi, ln2) without uncertainty or source | no G, ħ, ε0, μ0, me, σ; nothing versioned |
| `limina-dsl` | JSON laws, static dimension analysis, interval evaluation, check report, `solve_for`, `bound_curve`, two-model comparison | parameter units must be in the 13-symbol table; one scalar formula per option; `epistemic_status` has three values |
| library `data/limits` | betz, carnot, cop (draft, reference TODO) | no references, no other discipline |
| extraction | LLM reads claims and evidence, deterministic traceability checks; law pipeline for unknown claims (two models) | the catalog shown to the model is the 3 built-ins; unknown claims go straight to AI authoring |
| visualisation | `/v1/curve` (bound against one parameter), explorer, Carnot and COP apparatus | no surfaces, no uncertainty bands, no fields, no parameterless view |

Regression risks found: unit tests pin the 13-symbol table and reject `W/m^2`, `GW`, `Wh` (decision-engine design 3.4); `units_for_dimension` feeds `GET /v1/limits` metadata and its tests; the DSL accepts only table units. The public items `Verdict` and `Direction` are matched exhaustively by `limina-api`, so adding variants to them would break callers.

## 2. Principles kept

- The engine keeps its four verdicts. NO LAW IDENTIFIED, NEEDS INPUT and OUTSIDE MODEL DOMAIN are claim statuses of the analysis, not engine verdicts, so a missing law can never become IMPOSSIBLE.
- Public interfaces of the core only grow: new items, new optional trait methods with defaults; no variant is added to an enum matched by callers.
- Every built-in law carries its classification, validity domain, assumptions and a verifiable reference; constants come from CODATA 2022 (NIST) with their uncertainty.

## 3. Phases

1. **Units and constants (core).** Prefixed and compound SI units through a parser; the 13-symbol table and `units_for_dimension` unchanged; CODATA 2022 constants with value, unit, uncertainty and source in one versioned module; DSL constants taken from it, a measured constant evaluated as its interval `value ± u`. Decision D-25.
2. **Law model.** Classification (fundamental law, theorem, constitutive relation, theoretical bound, model, empirical law); equality laws as a two-sided band built from the measurement uncertainty; `outside_model_domain` when a validity condition fails; required references for built-ins.
3. **Library, by discipline.** One spec, one JSON definition and reference tests per law, in batches: mechanics and relativity; thermodynamics, statistical physics and information; electromagnetism and optics; quantum, atomic and nuclear; astrophysics and cosmology. Each law only when its equation, domain and reference are verified.
4. **Law identification.** The model chooses among the library first, with the law catalog in its prompt; two-model authoring only when no built-in law applies.
5. **Visualisation.** `/v1/surface` (bound over two parameters), uncertainty bands and validity zones on curves, parameterless comparison scale, field plots for laws that define a field; every plot computed by the engine.
6. **Vectors.** Vector quantities in the DSL where a law needs them (Lorentz force, field superposition).

Each phase: spec in `.kiro/specs`, reference tests, fmt, clippy, full test suite, commit, CI.
