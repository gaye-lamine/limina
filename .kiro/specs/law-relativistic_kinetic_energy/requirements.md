# Requirements: law-relativistic_kinetic_energy
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/relativistic_kinetic_energy.json`.

## Facts
- Name: Relativistic kinetic energy. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `J`.
- Equation: `m*c^2*(1/sqrt(1 - (v/c)^2) - 1)`
- Classification: `theorem`. Epistemic status: `theorem`.
- Validity domain: Kinetic energy (gamma - 1) m c^2 of a body of non-zero rest mass m at speed v < c in an inertial frame.
- Validity conditions: v < c
- Variables:
- `m`: Rest mass (kg, g)
- `v`: Speed in the inertial frame (m/s, km/s)

## Limits of application
- Kinetic energy only: the rest energy m c^2 is not included
- Free body in an inertial frame; rotation and internal energy are not included
- The 0.5 % tolerance covers the rounding of a stated figure, it is not a physical uncertainty

## References
- Rindler, W., Relativity: Special, General, and Cosmological, 2nd ed., Oxford University Press (2006)
- Taylor, E. F. and Wheeler, J. A., Spacetime Physics, 2nd ed., W. H. Freeman (1992)

## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound (at the prediction for an equality) is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound, or outside the tolerance band, is IMPOSSIBLE with the wording "inconsistent with limit relativistic_kinetic_energy under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.