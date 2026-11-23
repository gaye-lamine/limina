# Requirements: limit-cop

Status: v1, frozen. Base: `decision-engine` v5, `limit-carnot` v1 (both unchanged). Crate: `limina-core`, served by `limina-api`, shown by `web/`.

## Summary

The Carnot bound on the coefficient of performance (COP) of a heat pump or refrigerator. It is the first limit whose bound depends on an assumption, so it is the first that can give CONDITIONAL.

## Facts

- Limit id `cop`. Bounded quantity: coefficient of performance, dimensionless. Direction: Max bound.
- One assumption, `mode`, with two options:
  - `heating`: the bound is `Th/(Th-Tc)`;
  - `cooling`: the bound is `Tc/(Th-Tc)`.
- `Th` and `Tc` are the absolute temperatures (kelvin) of the hot and cold reservoirs.
- Two parameters, `Th` and `Tc`, typed `AbsoluteTemperature`. Valid only when `Th > Tc > 0`; otherwise the same typed error as Carnot, `InvalidTemperatureOrder`.
- No plausible domain for either parameter: a missing temperature gives INSUFFICIENT_DATA directly.
- Epistemic status: theorem (second law of thermodynamics).
- Reference: TODO, to be supplied by the author. No reference is invented here.

## Functional requirements

- **R-01** The limit declares id `cop`, direction Max, bounded kind dimensionless, the assumption `mode` with options `heating` and `cooling`, and parameters `Th` and `Tc` (kind `AbsoluteTemperature`, required, no plausible domain).
- **R-02** For valid point temperatures, `bound` returns `Th/(Th-Tc)` under `heating` and `Tc/(Th-Tc)` under `cooling`, computed in f64.
- **R-03** If the enclosures do not guarantee `Th > Tc > 0`, `bound` returns `InvalidTemperatureOrder` and computes no bound. No new error variant.
- **R-04** A missing temperature gives INSUFFICIENT_DATA with the missing parameters listed, produced by the engine.
- **R-05** Public documentation gives both formulas, the assumption and the TODO reference.
- **R-06** `limina-api` serves `cop` in `GET /v1/limits` and `POST /v1/evaluate`, with the same metadata fields as `betz` and `carnot`.
- **R-07** `web/` offers one example "COP, claim 14.5, Th 300 K, Tc 280 K" that shows CONDITIONAL with the satisfying configuration listed.

## Acceptance criteria

1. Golden (f64): `Th = 300 K`, `Tc = 280 K` gives 15.0 (heating) and 14.0 (cooling); `320 K`, `280 K` gives 8.0 and 7.0; `300 K`, `250 K` gives 6.0 and 5.0.
2. With `Th = 300 K`, `Tc = 280 K`: claim 10 gives COMPATIBLE; claim 16 gives IMPOSSIBLE; claim 14.5 with `mode` unconstrained gives CONDITIONAL with only `heating` in the satisfying set, tagged Certain; claim 14.5 constrained to `heating` gives COMPATIBLE; claim 14.5 constrained to `cooling` gives IMPOSSIBLE.
3. A missing temperature gives INSUFFICIENT_DATA; `Th <= Tc` gives `InvalidTemperatureOrder`.
4. Property: heating bound = cooling bound + 1 for any valid `Th > Tc > 0`, within the engine tolerances (`approx_eq`).
5. Property: both bounds decrease as `Th - Tc` grows at fixed `Tc`.
6. Property: for any claim at or below the bound, the engine never returns IMPOSSIBLE.
7. API handler tests: COP 14.5 with 300 K / 280 K and no constraint returns CONDITIONAL with the Certain/Depends tags; the same claim constrained to `cooling` returns IMPOSSIBLE.
8. Web component test: the COP example renders CONDITIONAL in the dashed-border style and lists the satisfying configuration.

## Out of scope

Any change to `engine.rs`, to the decision-engine, limit-betz or limit-carnot specs, Celsius conversion, the choice of a reference.
