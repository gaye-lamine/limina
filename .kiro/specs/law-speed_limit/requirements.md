# Requirements: law-speed_limit
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/speed_limit.json`.

## Facts
- Name: Speed limit of a massive body. Kind: max bound. Bounded quantity dimension: `m/s`.
- Equation: `c`
- Classification: `theoretical_bound`. Epistemic status: `theorem`.
- Validity domain: Speed of a body with non-zero rest mass, measured in an inertial frame, through space.
- Validity conditions: none in the DSL
- Variables:
- none

## Limits of application
- Does not apply to light or other massless particles, nor to phase or group velocities in a medium
- Does not apply to the recession velocity of distant galaxies or to coordinate speeds in curved spacetime
- A bound of special relativity: a body of non-zero mass stays strictly below c, and no technology is implied to approach it

## References
- Einstein, A., Zur Elektrodynamik bewegter Koerper, Annalen der Physik 322(10), 891-921 (1905), doi:10.1002/andp.19053221004
- Rindler, W., Relativity: Special, General, and Cosmological, 2nd ed., Oxford University Press (2006)

## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound (at the prediction for an equality) is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound, or outside the tolerance band, is IMPOSSIBLE with the wording "inconsistent with limit speed_limit under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.