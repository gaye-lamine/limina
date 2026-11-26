# Requirements: law-kepler_third
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/kepler_third.json`.

## Facts
- Name: Kepler's third law. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `s`.
- Equation: `2*pi*sqrt(a^3/(G*(m1 + m2)))`
- Classification: `model`. Epistemic status: `model_bound`.
- Validity domain: Orbital period T = 2 pi sqrt(a^3 / (G (m1 + m2))) of an isolated two-body system under Newtonian gravity, with semi-major axis a.
- Validity conditions: a > 0
- Variables:
- `a`: Semi-major axis of the relative orbit (m, km, au)
- `m1`: Mass of the first body (kg, g)
- `m2`: Mass of the second body (kg, g)

## Limits of application
- Two bodies only: perturbations by other bodies are ignored
- Newtonian: relativistic corrections are not included
- a is the semi-major axis of the relative orbit, not the distance to the centre of mass

## References
- Goldstein, H., Poole, C. and Safko, J., Classical Mechanics, 3rd ed., Addison-Wesley (2002)

## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound (at the prediction for an equality) is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound, or outside the tolerance band, is IMPOSSIBLE with the wording "inconsistent with limit kepler_third under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.