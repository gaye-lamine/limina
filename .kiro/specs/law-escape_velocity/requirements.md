# Requirements: law-escape_velocity
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/escape_velocity.json`.

## Facts
- Name: Escape velocity. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `m/s`.
- Equation: `sqrt(2*G*M/r)`
- Classification: `model`. Epistemic status: `model_bound`.
- Validity domain: Speed needed at distance r from the centre of a spherically symmetric, non-rotating mass M to reach infinity at rest, with no atmosphere and no other body.
- Validity conditions: r > 0
- Variables:
- `M`: Mass of the attracting body (kg, g)
- `r`: Distance from its centre (m, km)

## Limits of application
- Newtonian: not valid near a black hole
- Ignores atmospheric drag, rotation of the body and the pull of other bodies
- Speed at the instant of launch for a ballistic trajectory, not the speed a powered vehicle needs

## References
- Kleppner, D. and Kolenkow, R. J., An Introduction to Mechanics, 2nd ed., Cambridge University Press (2014)

## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound (at the prediction for an equality) is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound, or outside the tolerance band, is IMPOSSIBLE with the wording "inconsistent with limit escape_velocity under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.