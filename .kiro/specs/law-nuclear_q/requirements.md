# Requirements: law-nuclear_q
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/nuclear_q.json`.
## Facts
- Name: Energy released by a nuclear reaction. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `J`.
- Equation: `abs(m_initial - m_final)*c^2`
- Classification: `fundamental_law`. Epistemic status: `model_bound`.
- Validity domain: Magnitude of the energy Q = |sum of initial masses - sum of final masses| c^2 released or absorbed by a nuclear reaction, the masses being the total rest masses on each side.
- Validity conditions: m_initial > 0, m_final > 0
- Variables:
- `m_initial`: Total rest mass before the reaction (kg, g)
- `m_final`: Total rest mass after the reaction (kg, g)
## Limits of application
- m_initial and m_final are the sums of the rest masses on each side, supplied by the user; no mass is looked up or assumed
- Use the same kind of mass on both sides (all nuclear masses, or all neutral atomic masses with the electrons balanced), otherwise the electron terms are inconsistent
- The difference of two close masses loses precision: the masses must be known far better than the relative size of their difference
- Magnitude only: whether the reaction releases or absorbs energy is not decided
- Masses are in kg or g; atomic mass units must be converted by the user
## References
- Einstein, A., Ist die Traegheit eines Koerpers von seinem Energieinhalt abhaengig?, Annalen der Physik 323(13), 639-641 (1905), doi:10.1002/andp.19053231314
- Krane, K. S., Introductory Nuclear Physics, 3rd ed., Wiley (1988)
- CODATA 2022 recommended value of the speed of light in vacuum, https://physics.nist.gov/cgi-bin/cuu/Value?c
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit nuclear_q under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
