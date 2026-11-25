# Requirements: law-hydrogen_levels
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/hydrogen_levels.json`.
## Facts
- Name: Hydrogen binding energy of level n. Kind: equality (predicted value, relative tolerance 0.001). Bounded quantity dimension: `J`.
- Equation: `R_inf*h*c/n^2`
- Classification: `model`. Epistemic status: `model_bound`.
- Validity domain: Binding energy |E_n| = R_inf h c / n^2 of the level n of hydrogen in the non-relativistic model with an infinitely heavy nucleus. The claim is the magnitude of the binding energy, the energy E_n itself being negative.
- Validity conditions: n >= 1
- Variables:
- `n`: Principal quantum number (dimensionless)
## Limits of application
- Non-relativistic model: fine structure (relative order alpha^2), the Lamb shift and hyperfine structure are neglected
- Infinite nuclear mass: the reduced-mass correction for hydrogen (about 0.05 %) is covered by the 0.1 % tolerance; other one-electron ions need a nuclear charge Z that is not included
- n must be a positive integer; the core does not check that it is an integer
- Hydrogen only: multi-electron atoms are outside the model
- The magnitude of the binding energy is compared, the sign convention E_n < 0 is not decided
## References
- Bohr, N., On the constitution of atoms and molecules, Part I, Philosophical Magazine 26(151), 1-25 (1913), doi:10.1080/14786441308634955
- Griffiths, D. J. and Schroeter, D. F., Introduction to Quantum Mechanics, 3rd ed., Cambridge University Press (2018)
- CODATA 2022 recommended value of the Rydberg constant, https://physics.nist.gov/cgi-bin/cuu/Value?ryd
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit hydrogen_levels under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
