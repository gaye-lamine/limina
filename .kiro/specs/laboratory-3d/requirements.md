# Requirements: laboratory-3d

Status: v1, phase 1 (foundation) in progress. Base: `api-evaluate`, `api-limit-metadata`, `two-model-laws`, `web-experience`, `ui-design` steering (unchanged).

## Goal

A 3D laboratory where a person explores any law of the library, and any AI-written draft, through the bound computed by the engine. Nothing in the laboratory is written for one law: every control, every shape and every number comes from `GET /v1/limits` and the engine endpoints. The laboratory never decides a verdict.

## Non-regression rules

- **N-01** No Rust file changes. The laboratory uses only `GET /v1/limits`, `POST /v1/evaluate`, `POST /v1/curve`, `POST /v1/surface` and `POST /v1/solve`, as they are.
- **N-02** All 3D code lives in `web/src/app/laboratory/`. The route `/laboratory` is loaded with `loadComponent`, so Three.js is in a lazy chunk: the home page bundle does not grow and no existing test imports it.
- **N-03** `ui-design` and `product` steering apply: no software vocabulary on screen, a verdict always has its text label, WCAG 2.2 AA contrast, full keyboard use, visible focus, `prefers-reduced-motion` respected, colours from `web/src/tokens.css`.

## API contracts the laboratory relies on (verified in `crates/limina-api/src/`, 2026-11-26)

| Route | Request | Response | Constraints for the laboratory |
|---|---|---|---|
| `GET /v1/limits` | none | `limits[]`: `id`, `direction` (`Max`, `Min`, `Equal`), `title`, `formula`, `statement`, `claim_unit`, `assumptions[]` (`id`, `options[]`), `parameters[]` (`id`, `kind`, `required`, `units[]`), `assumption_labels`, `option_labels`, `parameter_labels`, optional `status` (`draft`), `agreement`, `classification`, `references`, `validity_domain`, `limitations` | 35 built-in laws plus the drafts authored since the server started. Units listed per parameter are the declared ones; the core laws (betz, carnot, cop) list `K` only for temperatures. |
| `POST /v1/evaluate` | `limit_id`, `claim {value, unit, uncertainty?}`, `constraints {assumption: [options]}`, `parameters {id: {value, unit}}`; unknown fields refused | `verdict`, `wording`, `bounds {permissive, restrictive: {b_min, b_max, margin}}` (SI), `configurations[]`, `missing_parameters[]` | Units must be parsed by the core: no °C, ft or solar masses here (that conversion exists only when a text is read). A temperature parameter takes `K` only. Betz without `reference_area`, and Eddington without its accretion geometry, answer 422 `NotApplicable` (no verdict). Carnot gives IMPOSSIBLE only with `conversion_process: [heat_engine_cycle]`; without it the answer is CONDITIONAL. An equality law answers against its tolerance band, or 422 `OutsideModelDomain` when a validity condition fails. A draft is evaluated with its formula status open: never IMPOSSIBLE. A missing parameter gives INSUFFICIENT_DATA. |
| `POST /v1/curve` | `limit_id`, `constraints`, `parameters` (every one but the free one, SI or a core unit), `free_parameter`, `range? [lo, hi]` (SI), `points?` (2 to 1000, default 101), `uncertainties?`, `spacing?` (`linear`, `log`, `auto`); unknown fields refused | `direction`, `range`, `points[] {x, b_min, b_max, decisive}` (null where undefined), `kind` (`bound` or `equality`), `lower?`, `band?`, `validity_zones[]`, `spacing` | Without `range`, the parameter's plausible domain or test range is used, else 422 `NoSearchRange`. An undeclared parameter name is refused. Carnot is drawn for a heat engine cycle when the conversion process is left open. |
| `POST /v1/surface` | `limit_id`, `constraints`, `parameters`, `x_parameter`, `y_parameter`, `x_range?`, `y_range?`, `samples?` (per axis, at most 50, default 30); unknown fields refused | `direction`, `x[]`, `y[]`, `values[y][x]` (null where undefined) | Ranges default to the test ranges. An equality law is drawn as its upper band edge only. |
| `POST /v1/solve` | `limit_id`, `claim`, `constraints`, `parameters` (all but the free one), `free_parameter`, `range?`; unknown fields refused | `threshold`, `direction` (`at least`, `at most`), `unit`, `range`, `below`, `above`, `verdict_below`, `verdict_above`, `sentence` | Only bound laws written in the DSL (the 18 max and min laws, betz, carnot and cop included) and drafts: the 17 equality laws answer 422 `UnknownLimit`, and a law with no parameter has nothing to solve. A bound that is not monotone on the range gives 422 `NotMonotone`; no crossing gives `NoThresholdInRange`; a missing fixed value gives `MissingFixedParameter`. The values are SI. |

## Functional requirements

- **R-01** A **Laboratory** link in the top bar, next to Library and How it works, opens `/laboratory`; the analysis in progress is kept when coming back.
- **R-02** A law selector filled from `GET /v1/limits`: every built-in law and every draft, grouped by direction, searchable by title, keyboard operable. Drafts are labelled as written by AI and not validated.
- **R-03** "Inspect in 3D Lab" on an analysed claim opens the laboratory with that claim's law, value, unit, parameters and stated assumptions already set.
- **R-04** Controls generated from the selected law's metadata: one slider per parameter (range from the curve or surface response, logarithmic when it spans three decades or more) with a unit chooser limited to the declared units; the claimed value with its unit (the claim unit and the core units of its dimension); one chooser per assumption with more than one option, with "not stated" as a choice. Labels come from `parameter_labels`, `assumption_labels` and `option_labels`.
- **R-05** The topology follows the number of parameters:
  - two or more: a **3D topological surface** of the bound over two chosen parameters (`/v1/surface`), the claim as a horizontal plane, the region inconsistent with the law shaded;
  - one: a **3D phase corridor** (`/v1/curve`), the bound as a ribbon along the parameter, the equality band or the uncertainty band as its walls, the claim as a line through it;
  - none: a **spherical confinement chamber** (`/v1/evaluate`): the bound as the chamber wall, the claim as a probe whose radius is the claim over the bound.
- **R-06** A physical skin chosen from the declared dimensions only: thermal (a temperature parameter), gravitational and velocity (a mass, a GM or a speed), wave and optics (a wavelength, a frequency, an angle), energy (an energy or a power), neutral otherwise. The skin changes materials and labels, never a number.
- **R-07** "Find Breaking Point" calls `/v1/solve` for the chosen parameter and places a marker at the threshold, with the engine's sentence and the verdicts on either side. Where the route cannot answer (equality law, no parameter, not monotone), the action says so in plain words and is not offered as broken.
- **R-08** A status panel (HUD) states, in words, the verdict with its label, the bound, the claim and the ratio, from the last engine answer. Every 3D element has a text alternative; the verdict is never shown by colour alone.
- **R-09** Keyboard: the view rotates, zooms and resets from the keyboard; every control is reachable by Tab; focus is visible. Motion stops under `prefers-reduced-motion`.
- **R-10** Every WebGL resource (renderer, geometries, materials, textures, controls, observers, animation frame) is released when the laboratory closes.

## Phase 1 acceptance (this task)

- The route, the top bar link, the lazy chunk and a working scene: dark observatory lighting from the design tokens, a metric grid, smooth orbit controls (pointer and keyboard), a status panel that lists how many laws the library holds, and full release of WebGL resources in `ngOnDestroy`.
- Pure, tested helpers for the topology choice (R-05) and the physical skin (R-06), driven by the metadata only.
- `npm test`, `npm run lint`, `npm run build` pass, with the existing tests unchanged.
