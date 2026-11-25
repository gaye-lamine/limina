# Requirements: law-tsiolkovsky
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/tsiolkovsky.json`.

## Facts
- Name: Tsiolkovsky rocket equation. Kind: max bound. Bounded quantity dimension: `m/s`.
- Equation: `ve*ln(max(m0, mf)/min(m0, mf))`
- Classification: `theoretical_bound`. Epistemic status: `model_bound`.
- Validity domain: Non-relativistic rocket with constant effective exhaust speed ve, burning from initial mass m0 to final mass mf, with no external force.
- Validity conditions: none in the DSL
- Variables:
- `ve`: Effective exhaust speed relative to the rocket (m/s, km/s)
- `m0`: Initial total mass (kg, g)
- `mf`: Final total mass after the burn (kg, g)

## Limits of application
- Gravity, drag and any other external force lower the achievable change of speed below this value
- Non-relativistic: not valid when the speed approaches c
- The masses are ordered in the formula (larger over smaller), so that swapped inputs never give a false IMPOSSIBLE; m0 is the mass before the burn
- A bound of an idealized rocket, not a figure that a real vehicle reaches

## References
- Sutton, G. P. and Biblarz, O., Rocket Propulsion Elements, 9th ed., Wiley (2016)
- Kleppner, D. and Kolenkow, R. J., An Introduction to Mechanics, 2nd ed., Cambridge University Press (2014)

## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound (at the prediction for an equality) is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound, or outside the tolerance band, is IMPOSSIBLE with the wording "inconsistent with limit tsiolkovsky under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.