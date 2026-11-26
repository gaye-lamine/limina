# Requirements: web-experience (v1, frozen)
Status: v1, frozen. Amends `web-evaluate` v1 (not modified). Based on `api-evaluate` v1 as amended by `api-limit-metadata` v1. UI rules: `.kiro/steering/ui-design.md`.

## Scope
Rework of the Evaluate page in `web/`: opening state, units from metadata, examples, verdict sheet, bound-scale chart, guided recovery, live updates, "How it works", layout. No verdict is computed in the browser. No extraction, no LLM, no external link, no external font or CDN.

## Requirements
- R-01 Opening state: the result pane shows a legend of the four verdicts, each in its own style (`verdict--*` classes) with a one-line meaning. No error block. An error block appears only after a real failed request.
- R-02 Units are never typed. The claim unit is `claim_unit` of the selected limit; parameter units are `parameters[].units` (`GET /v1/limits`). A single unit is shown as read-only text next to the value. Several units give a select. The web app holds no table of units.
- R-03 A "Try an example" list of four one-click claims, each labelled "illustrative claim, not from a real company":
  1. Betz 0.70, constrained to `unshrouded_rotor` (expected IMPOSSIBLE).
  2. Betz 0.45, constrained to `unshrouded_rotor` (expected COMPATIBLE).
  3. Carnot 0.60, Th = 600 K, Tc = 300 K (expected IMPOSSIBLE).
  4. Carnot 0.40, no temperatures (expected INSUFFICIENT_DATA).
  Clicking one fills the form and evaluates. The expected verdicts are what the engine returns; the browser does not compute them.
- R-04 Verdict sheet: the verdict as a large uppercase label in its own style, the API `wording` below it in the serif stack, then bound, claim and margin in tabular monospace.
- R-05 Bound-scale chart, inline SVG, black, white and grey only: horizontal axis from 0 to about 1.25 times the larger of claim and bound; a thick vertical line at the bound with its value labelled below; a thin line at the claim with its value labelled above; the region beyond the bound filled with a grey diagonal hatch. The geometry is a pure function with unit tests: a claim above the bound lies inside the hatched region, a claim below it does not. The chart has a text alternative with the same information.
- R-06 Guided recovery: on `NotApplicable` for Betz, two buttons "Evaluate as unshrouded rotor" and "Evaluate as shrouded, total capture area" set the `reference_area` constraint and evaluate again.
- R-07 Live updates: after the first evaluation, changing the claim value or an assumption re-evaluates automatically (debounced); the verdict and the chart update.
- R-08 A short "How it works" block: the verdict is computed by a deterministic engine, never by a language model; the language model, when used, only reads the numbers from the text. A line credits ONTO Limits as prior art. No external link.
- R-09 Layout: strong typographic hierarchy, generous white space, bounded content width, two columns from 960px and one column below, restyled form controls, no icon, no emoji, no color.
- R-10 Tests: component tests for the legend, examples, guided recovery and live update; unit tests for the chart geometry; all existing tests keep passing.

## Decisions on open points
- A. Bounds that are a range (`b_min` differs from `b_max`, in either enclosure): the thick line and the hatch start at the largest `b_max` of the two enclosures, so the hatch only covers values excluded in every configuration. A light grey band spans from the smallest `b_min` to that line, and the label reads "low to high".
- B. No bounds (Carnot without temperatures): no chart is drawn. A text line says no bound could be computed and lists the missing parameters.
- C. Metadata mismatch: none after `api-limit-metadata`. `bounds` values are in SI base units and the claim is in `claim_unit`; the chart assumes they are the same, which holds for `dimensionless`. A limit whose claim unit is not an SI base unit needs a conversion, not yet needed.

## Revision 2026-11-26 (audit of 26/11)
- R-11 While an analysis runs, a "Cancel analysis" button stops it: the request in flight is aborted (RxJS `takeUntil`, which cancels the HTTP request), no further paragraph is sent, the claims read so far are reviewed, and a notice says so.
- R-12 A paragraph stopped by the analysis deadline says so in plain words ("Checking this paragraph took too long, so it was stopped. Try a shorter paragraph, or try again later.") and is not retried; no code or status number is shown.
- R-13 A CONDITIONAL sentence names the options under which the claim holds, leaving out assumptions with one option only (Carnot's cycle model).
