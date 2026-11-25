# Requirements: law-faraday_emf
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/faraday_emf.json`.
## Facts
- Name: Faraday induced EMF. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `V`.
- Equation: `n_turns*area*abs(dB)/dt`
- Classification: `fundamental_law`. Epistemic status: `model_bound`.
- Validity domain: Magnitude of the EMF induced in a coil of n_turns turns of area `area`, fixed and perpendicular to a uniform field whose magnitude changes by dB at a constant rate over a time dt.
- Validity conditions: dt > 0
- Variables:
- `n_turns`: Number of turns of the coil (dimensionless)
- `area`: Area of one turn (m^2, cm^2)
- `dB`: Change of the field magnitude (T, mT)
- `dt`: Duration of the change (s, ms)
## Limits of application
- Fixed loop, uniform field perpendicular to the loop, constant rate of change: an average EMF over dt otherwise
- Magnitude only: the sign (Lenz's law) is not decided
- A moving or rotating loop, or a field not perpendicular to the loop, needs the flux derivative in general
- The 0.5 % tolerance covers the rounding of a stated figure, it is not a physical uncertainty
## References
- Griffiths, D. J., Introduction to Electrodynamics, 4th ed., Cambridge University Press (2017)
- Jackson, J. D., Classical Electrodynamics, 3rd ed., Wiley (1999)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit faraday_emf under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
