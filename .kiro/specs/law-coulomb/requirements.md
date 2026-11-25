# Requirements: law-coulomb
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/coulomb.json`.
## Facts
- Name: Coulomb force. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `N`.
- Equation: `abs(q1*q2)/(4*pi*eps0*r^2)`
- Classification: `fundamental_law`. Epistemic status: `model_bound`.
- Validity domain: Magnitude of the electrostatic force between two point charges at rest in vacuum, separated by r.
- Validity conditions: r > 0
- Variables:
- `q1`: Magnitude of the first charge (C, mC, uC, nC)
- `q2`: Magnitude of the second charge (C, mC, uC, nC)
- `r`: Distance between the charges (m, cm, mm)
## Limits of application
- Point charges at rest in vacuum; moving charges need the full Maxwell equations
- In a dielectric medium the force is divided by the relative permittivity, which is not included
- Charges are given as magnitudes: the sign (attraction or repulsion) is not decided
- Extended charge distributions are approximated by their centres, which is only valid when r is much larger than their size
- The uncertainty of eps0 (CODATA 2022) is propagated by the engine, apart from the tolerance
## References
- Jackson, J. D., Classical Electrodynamics, 3rd ed., Wiley (1999)
- Griffiths, D. J., Introduction to Electrodynamics, 4th ed., Cambridge University Press (2017)
- CODATA 2022 recommended value of the vacuum electric permittivity, https://physics.nist.gov/cgi-bin/cuu/Value?ep0
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit coulomb under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
