# Requirements: limit-carnot
Status: v1, frozen. Base: `decision-engine` v5 and `limit-betz` v1 (both unchanged). Crate: `limina-core`.
## Summary
The Carnot limit bounds the thermal efficiency of a heat engine. It is the first limit with numeric parameters, so it exercises missing-parameter handling.
## Facts
- Bounded quantity: thermal efficiency of a heat engine, dimensionless. Direction: Max bound. Bound: `1 - Tc/Th`, with `Th` and `Tc` the absolute temperatures (kelvin) of the hot and cold reservoirs.
- Two parameters, `Th` and `Tc`, typed `AbsoluteTemperature`. Valid only when `Th > Tc > 0`, otherwise a typed error.
- No plausible domain is declared for either parameter. A missing temperature gives INSUFFICIENT_DATA directly, with the missing parameters listed (decision-engine D-04, D-16).
- One assumption, `cycle_model`, with one option: `ideal_reversible_two_reservoirs` (the bound is the Carnot bound).
- Epistemic status: theorem (second law of thermodynamics).
- Reference: TODO, to be supplied by the author. No reference is invented here.
- Celsius is never converted silently: temperatures arrive as `AbsoluteTemperature`.
## Functional requirements
- **R-01** The limit declares id `carnot`, direction Max, bounded kind dimensionless, the assumption `cycle_model` with its single option, and parameters `Th` and `Tc` (kind `AbsoluteTemperature`, required, no plausible domain).
- **R-02** For valid `Th > Tc > 0`, `bound` returns the point interval `1 - Tc/Th` computed in f64.
- **R-03** If the enclosures do not guarantee `Th > Tc > 0`, `bound` returns a typed error and computes no bound. An existing variant is reused if one fits; otherwise ONE new variant is added (see design 2).
- **R-04** A missing temperature gives INSUFFICIENT_DATA with the missing parameters listed, produced by the engine, not by the limit.
- **R-05** The public documentation gives the formula, the assumption and the TODO reference, and states that Celsius is not converted.
## Acceptance criteria
1. Golden: `Th = 600 K`, `Tc = 300 K` gives exactly 0.5; `Th = 500 K`, `Tc = 250 K` gives 0.5. Both computed as `1.0 - tc / th`.
2. Claim 0.30 with `Th = 600 K`, `Tc = 300 K` gives COMPATIBLE, or CONDITIONAL if the engine rules dictate.
3. Claim 0.60 with `Th = 600 K`, `Tc = 300 K` gives IMPOSSIBLE.
4. Any claim with a missing temperature gives INSUFFICIENT_DATA listing the missing parameters.
5. `Th <= Tc` gives the typed error.
6. Property: the bound is strictly below 1 for any valid `Tc > 0`.
7. Property: the bound is monotone increasing in `Th` at fixed `Tc`, and decreasing in `Tc` at fixed `Th`, checked against an independent oracle.
8. Property: for any claim at or below the bound, the engine never returns IMPOSSIBLE.
9. Public documentation of R-05 is present, TODO reference clearly marked.
## Out of scope
Extraction pipeline, unit conversion from Celsius, any change to `engine.rs`, the decision-engine spec or the limit-betz spec, the choice of a reference.
