# Requirements: law-heisenberg
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/heisenberg.json`.
## Facts
- Name: Heisenberg position-momentum relation. Kind: min bound. Bounded quantity dimension: `m`.
- Equation: `hbar/(2*dp)`
- Classification: `theorem`. Epistemic status: `theorem`.
- Validity domain: Standard deviations dx and dp of position and momentum along the same axis, for any quantum state of a particle: dx * dp >= hbar / 2 (Kennard 1927). The law bounds dx from below for a given dp.
- Validity conditions: none
- Variables:
- `dp`: Standard deviation of the momentum along the axis (kg*m/s)
## Limits of application
- dx and dp are standard deviations over an ensemble of identically prepared systems, not the error of one measurement
- The bound is for the same axis; different axes are independent
- A lower bound on the product: real states, such as a non-Gaussian packet, have a larger product
- The core checks no condition dp > 0 here, a non-positive dp is not meaningful and gives no verdict of its own
- Not a statement about measurement disturbance (Ozawa and others show that is a different relation)
## References
- Heisenberg, W., Ueber den anschaulichen Inhalt der quantentheoretischen Kinematik und Mechanik, Zeitschrift fuer Physik 43(3-4), 172-198 (1927), doi:10.1007/BF01397280
- Kennard, E. H., Zur Quantenmechanik einfacher Bewegungstypen, Zeitschrift fuer Physik 44, 326-352 (1927), doi:10.1007/BF01391200
- Griffiths, D. J. and Schroeter, D. F., Introduction to Quantum Mechanics, 3rd ed., Cambridge University Press (2018)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or above the bound is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the bound is IMPOSSIBLE with the wording "inconsistent with limit heisenberg under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
