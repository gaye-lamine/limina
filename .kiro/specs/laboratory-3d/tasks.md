# Tasks: laboratory-3d

## Phase 1: foundation (this task)
- [x] 1.1 Verify the contracts of `/v1/limits`, `/v1/evaluate`, `/v1/curve`, `/v1/surface`, `/v1/solve` and record their constraints (requirements, table).
- [x] 1.2 Install `three` 0.187.0 and `@types/three` 0.187.0, pinned.
- [x] 1.3 Router with one lazy route `/laboratory` (`loadComponent`); the workbench stays mounted; a Laboratory link in the top bar.
- [x] 1.4 `laboratory/scene.ts`: renderer, camera, observatory lights and metric grid from the design tokens, orbit controls (pointer and keyboard), render loop, resize, reduced motion, full release.
- [x] 1.5 `laboratory/laboratory.ts`: page, status panel with the number of laws from `GET /v1/limits`, text alternative, Reset view.
- [x] 1.6 `laboratory/topology.ts` and `laboratory/skin.ts` (pure) with unit tests over the metadata of the test fixtures.
- [x] 1.7 `npm test`, `npm run lint`, `npm run build`; existing tests unchanged.

## Phase 2: dynamic controls
- [x] 2.1 Law selector (built-in and drafts), grouped by direction, searchable, keyboard operable.
- [x] 2.2 Generated parameter sliders, unit choosers, claim input, assumption choosers.
- [x] 2.3 Debounced, cancelled requests; status panel from `/v1/evaluate`.

## Phase 3: topologies and skins
- [x] 3.1 Surface from `/v1/surface`, claim plane, inconsistent region shaded and hatched.
- [x] 3.2 Corridor from `/v1/curve`, bands and validity zones.
- [x] 3.3 Chamber from `/v1/evaluate`.
- [x] 3.4 Physical skins.

## Phase 4: actions and links
- [x] 4.1 Find Breaking Point (`/v1/solve`), with the cases the route cannot answer.
- [x] 4.2 Inspect in 3D Lab from the workbench.
- [x] 4.3 Playwright: lab screens at the three sizes, DOM checks, keyboard path; WebGL release checked by navigating away and back. (1920x1080, 1440x900, 768x1024, 390x844; context count checked on /laboratory, /, /laboratory.)

## Phase 5: dual view (physical apparatus and mathematical boundary)
- [x] 5.1 Segmented control Physical apparatus (default) / Mathematical boundary, radio group with arrow keys; the stage's text alternative describes the active view.
- [x] 5.2 `apparatus.ts` (pure): `apparatusFor(limit)` from units, parameter kinds and labels only, with the drawing quantities; unit tests over the four benches and an unseen draft.
- [x] 5.3 `apparatus-view.ts`: energy, field, optics and quantum benches, driven by the same controls, Find Breaking Point, Dual Probe and Inspect in 3D Lab.
- [x] 5.4 Switching releases the other view; e2e checks the GPU geometry count over four round trips and the WebGL context on /laboratory, /, /laboratory.
- [x] 5.5 Test runner: `vitest-base.config.ts` (two workers, 15 s timeouts) via `runnerConfig`; `claimOf` and `contrast` moved out of spec files into `testing-fixtures.ts` and `testing-contrast.ts` (importing a spec from another spec made Vitest report "No test suite found" and run its tests twice).
- [x] 5.6 Quantum bench: decaying population (343 nuclei, lit share = claimed remaining over the initial count, rings for the claimed and expected fractions); confinement plates outlined.
- [x] 5.7 Field bench: a claimed length gets a measured-radius ring around the reference-radius sphere, its label outside both radii; speeds keep the path labelled Escapes or Captured.

## Phase 6, pass A: direct manipulation, glow, sound
- [x] 6.1 Draggable handles (`drag-map.ts`, `pointer.ts`): boundary probe and chamber core; walls, ring, field probe, hot source, aperture; orbit off while dragging; grab cursors; keyboard unchanged; unit tests of the maps; e2e drags of the Carnot probe and a Heisenberg wall.
- [x] 6.2 Glow (`glow.ts`): bloom between render and output passes, labels in an overlay drawn after it, direct rendering without float targets, disposal; WebGL round trip still releases its context.
- [x] 6.3 Sound (`lab-audio.ts`): off by default, keyboard toggle, tone, thud on IMPOSSIBLE, ping on breaking point; context closed when off and on leaving; unit tests with a fake context, e2e count of created and closed contexts.

## Phase 6, pass B: forge and stress test
- [x] 6.4 Forge a new law (`forge.ts`, `lab-forge.ts`, `AnalysisService.author`): panel, examples, plain progress and refusals, draft opened with apparatus and boundary; unit tests; e2e refusal (mocked 422) and success (mocked draft routes).
- [x] 6.5 System stress test (`cascade.ts`, `lab-cascade.ts`): two to four bound laws, System load 20-150 %, engine verdicts and ratios, First breaking limit with sound, show any law in 3D; "Stress-test all laws in 3D Lab" from the analysis; unit tests; e2e in the laboratory and from a mocked analysis.
