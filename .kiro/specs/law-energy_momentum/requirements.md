# Requirements: law-energy_momentum
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/energy_momentum.json`.

## Facts
- Name: Relativistic energy-momentum relation. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `J`.
- Equation: `sqrt((p*c)^2 + (m*c^2)^2)`
- Classification: `theorem`. Epistemic status: `theorem`.
- Validity domain: Total energy of a free body or particle of rest mass m and momentum magnitude p in an inertial frame, in special relativity.
- Validity conditions: m >= 0
- Variables:
- `p`: Momentum magnitude (kg*m/s)
- `m`: Rest mass (kg, g)

## Limits of application
- Total energy including rest energy, not kinetic energy
- Free body: no potential energy, no field energy
- The 0.5 % tolerance covers the rounding of a stated figure, it is not a physical uncertainty

## References
- Rindler, W., Relativity: Special, General, and Cosmological, 2nd ed., Oxford University Press (2006)
- Taylor, E. F. and Wheeler, J. A., Spacetime Physics, 2nd ed., W. H. Freeman (1992)

## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound (at the prediction for an equality) is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound, or outside the tolerance band, is IMPOSSIBLE with the wording "inconsistent with limit energy_momentum under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.