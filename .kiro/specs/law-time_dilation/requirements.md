# Requirements: law-time_dilation
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/time_dilation.json`.

## Facts
- Name: Time dilation. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `s`.
- Equation: `tau/sqrt(1 - (v/c)^2)`
- Classification: `theorem`. Epistemic status: `theorem`.
- Validity domain: Interval measured by an inertial observer for a clock moving at constant speed v < c, given its proper time tau.
- Validity conditions: v < c
- Variables:
- `tau`: Proper time measured by the moving clock (s, ms, us, µs, ns)
- `v`: Speed of the clock in the observer frame (m/s, km/s)

## Limits of application
- Constant velocity in an inertial frame; acceleration and gravitational time dilation are not included
- The 0.5 % tolerance covers the rounding of a stated figure, it is not a physical uncertainty

## References
- Einstein, A., Zur Elektrodynamik bewegter Koerper, Annalen der Physik 322(10), 891-921 (1905), doi:10.1002/andp.19053221004
- Rindler, W., Relativity: Special, General, and Cosmological, 2nd ed., Oxford University Press (2006)

## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound (at the prediction for an equality) is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound, or outside the tolerance band, is IMPOSSIBLE with the wording "inconsistent with limit time_dilation under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.