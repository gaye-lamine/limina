# Design: laboratory-3d

## 1. Placement

- `web/src/app/app.routes.ts`: one route, `{ path: 'laboratory', loadComponent: () => import('./laboratory/laboratory').then((m) => m.Laboratory) }`. The router is provided in `app.config.ts` with `provideRouter(routes)`.
- `app.html` keeps `<app-workbench>` mounted (hidden on `/laboratory`) so that `App`'s `viewChild(Workbench)`, the command palette and an analysis in progress keep working, and adds `<router-outlet />` for the laboratory. The top bar link is an anchor with `routerLink="/laboratory"` and `aria-current` when active.
- Everything else lives in `web/src/app/laboratory/`:
  - `laboratory.ts` / `.html`: the page (controls, status panel, the scene host);
  - `scene.ts`: the Three.js observatory (renderer, camera, lights, grid, orbit controls, render loop, keyboard orbit, release);
  - `topology.ts`: which shape a law gets, from its parameters (R-05), pure;
  - `skin.ts`: which physical skin a law gets, from its declared dimensions (R-06), pure;
  - `theme.ts`: reads the design tokens (`--bg`, `--surface-*`, `--accent`, `--v-*`, `--text-*`) from the document at run time, so no colour is written in the 3D code.
- Strings live in `web/src/app/strings.ts` (object `LAB`), as the steering requires.

## 2. Data flow (phases 2 to 4)

1. `GET /v1/limits` fills the selector. Selecting a law builds a form model: parameters (id, label, declared units, kind), the claim (value, `claim_unit`), assumptions with more than one option.
2. The topology is `surface` (two parameters or more; the user picks the two axes, the others are sliders), `corridor` (one), `chamber` (none).
3. Requests are debounced (150 ms) and cancelled on change (`switchMap`):
   - surface: `/v1/surface` with `samples` 30 (at most 50), the other parameters fixed;
   - corridor: `/v1/curve` with `points` 101 and `spacing: auto`, plus `uncertainties` when asked;
   - every change also calls `/v1/evaluate` for the status panel (the verdict always comes from there).
4. "Find Breaking Point": `/v1/solve` with the chosen free parameter; offered only for `Max` and `Min` laws with at least one parameter; a 422 answer is shown in words (not monotone, no crossing in the range).
5. "Inspect in 3D Lab": the workbench passes the claim through router state (`limit_id`, value and unit, parameters, constraints); the laboratory reads it once and fills the form.

## 3. Geometry

- Axis values are SI, as the engine returns them; axes use a logarithmic scale when the range spans three decades or more (same rule as the explorer, `graph-geometry.ts`). Labels show the chosen display unit.
- Surface: a `BufferGeometry` grid from `values[y][x]`; a null cell is a hole (no triangle). The claim is a translucent plane at the claimed value; the side inconsistent with the law (above for `Max`, below for `Min`, outside the band for `Equal`) uses the impossible token colour plus a hatched texture, so it is not told by colour alone.
- Corridor: a ribbon swept along x from `points[].decisive`; walls from `lower`/`band` when present; validity zones as greyed slabs.
- Chamber: a wireframe sphere of radius 1 for the bound; the claim a solid sphere of radius `claim / bound` (from the engine margin); inside or outside is also written in the status panel.

## 4. Accessibility

- The canvas sits in a region with `role="img"` and an `aria-label` that states what is drawn; the status panel is `aria-live="polite"` text.
- Keyboard on the focused scene: arrows rotate, `+`/`-` zoom, `0` resets; a visible "Reset view" button does the same.
- `prefers-reduced-motion`: no damping inertia, no auto-rotation, no animated transitions.

## 5. Release of resources

`ngOnDestroy` cancels the animation frame, disconnects the `ResizeObserver`, removes listeners, disposes the orbit controls, walks the scene and disposes every geometry, material and texture, disposes the renderer, forces the context loss and removes the canvas.

## 6. Decisions

- D-01 The laboratory sends units the core parses only (the declared ones and the core units of the same dimension). The reading-time conversions of `limina-extract` (°C, ft, solar masses) are not available through these routes, and the Rust code is not changed.
- D-02 The verdict shown is the one of `/v1/evaluate`, never derived from the surface or the curve.
- D-03 Three.js is pinned at 0.187.0 with `@types/three` 0.187.0 (no peer dependency; Angular 21 unaffected).
