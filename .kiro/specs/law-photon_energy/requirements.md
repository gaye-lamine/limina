# Requirements: law-photon_energy
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/photon_energy.json`.
## Facts
- Name: Photon energy from frequency. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `J`.
- Equation: `h*f`
- Classification: `fundamental_law`. Epistemic status: `model_bound`.
- Validity domain: Energy E = h f of one photon of frequency f in vacuum (Planck-Einstein relation).
- Validity conditions: f > 0
- Variables:
- `f`: Frequency of the photon (Hz, kHz, MHz, GHz, THz, PHz)
## Limits of application
- Energy of a single photon, not of a beam: a beam carries N times this energy
- The frequency is the one of the photon in the frame of the observer
- The 0.5 % tolerance covers the rounding of a stated figure, it is not a physical uncertainty
## References
- Griffiths, D. J. and Schroeter, D. F., Introduction to Quantum Mechanics, 3rd ed., Cambridge University Press (2018)
- CODATA 2022 value of the Planck constant (exact in the 2019 SI), https://physics.nist.gov/cgi-bin/cuu/Value?h
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit photon_energy under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
