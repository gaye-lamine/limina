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
- [ ] 2.1 Law selector (built-in and drafts), grouped by direction, searchable, keyboard operable.
- [ ] 2.2 Generated parameter sliders, unit choosers, claim input, assumption choosers.
- [ ] 2.3 Debounced, cancelled requests; status panel from `/v1/evaluate`.

## Phase 3: topologies and skins
- [ ] 3.1 Surface from `/v1/surface`, claim plane, inconsistent region shaded and hatched.
- [ ] 3.2 Corridor from `/v1/curve`, bands and validity zones.
- [ ] 3.3 Chamber from `/v1/evaluate`.
- [ ] 3.4 Physical skins.

## Phase 4: actions and links
- [ ] 4.1 Find Breaking Point (`/v1/solve`), with the cases the route cannot answer.
- [ ] 4.2 Inspect in 3D Lab from the workbench.
- [ ] 4.3 Playwright: lab screens at the three sizes, DOM checks, keyboard path; WebGL release checked by navigating away and back.
