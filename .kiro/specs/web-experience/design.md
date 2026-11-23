# Design: web-experience (v1, frozen)
Status: v1, frozen.

- `api.models.ts`: `LimitInfo.claim_unit`, `ParameterSpec.units`.
- `chart-geometry.ts`: pure functions `boundScale` (axis, x positions, hatch rectangle, `claimInHatch`) and `summarizeBounds` (overall low and high bound, margin hull from `bounds`). No Angular import.
- `bound-chart.ts`: standalone component rendering the SVG from `boundScale`; `<title>`, `<desc>` and a visible caption carry the text alternative.
- `result-panel`: legend when no result and no failure; verdict sheet; chart or "no bound" line; error block only for a failure; recovery buttons for `NotApplicable` when `limitId` is `betz` (output `recover` with the option id).
- `examples.ts`: the four examples as data (limit, claim text, constraints, parameter values). Units come from the metadata.
- `evaluate-form`: claim unit and single parameter units are read-only inputs; several units give a select. `applyExample`, `recover`, and a debounced live re-evaluation (300 ms) started after the first evaluation by a change of claim value or assumption. A newer request cancels the previous one. A live re-evaluation with an invalid claim is skipped silently.
- `strings.ts`: all new text.
- `styles.css`: type scale, restyled controls (`appearance: none`, 1px border, focus ring), serif wording, hatch and chart classes, two columns from 960px.
