# Requirements: law-newton_gravity
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/newton_gravity.json`.

## Facts
- Name: Newtonian gravitational force. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `N`.
- Equation: `G*m1*m2/r^2`
- Classification: `model`. Epistemic status: `model_bound`.
- Validity domain: Force between two point masses or spherically symmetric bodies at centre distance r, weak field and speeds much below c.
- Validity conditions: r > 0
- Variables:
- `m1`: First mass (kg, g)
- `m2`: Second mass (kg, g)
- `r`: Distance between the centres (m, km, au)

## Limits of application
- An approximation of general relativity: not valid near compact objects or for precision orbital work
- r is the distance between centres; for non-spherical bodies the formula is approximate
- The uncertainty of G (CODATA 2022) is propagated by the engine, apart from the tolerance

## References
- Goldstein, H., Poole, C. and Safko, J., Classical Mechanics, 3rd ed., Addison-Wesley (2002)
- CODATA 2022 recommended value of the Newtonian constant of gravitation G, https://physics.nist.gov/cgi-bin/cuu/Value?bg

## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound (at the prediction for an equality) is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound, or outside the tolerance band, is IMPOSSIBLE with the wording "inconsistent with limit newton_gravity under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.