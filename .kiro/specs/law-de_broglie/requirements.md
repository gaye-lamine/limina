# Requirements: law-de_broglie
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/de_broglie.json`.
## Facts
- Name: de Broglie wavelength. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `m`.
- Equation: `h/p`
- Classification: `fundamental_law`. Epistemic status: `model_bound`.
- Validity domain: Wavelength associated with a free particle of momentum magnitude p: lambda = h / p.
- Validity conditions: p > 0
- Variables:
- `p`: Magnitude of the momentum of the particle (kg*m/s)
## Limits of application
- p is the relativistic momentum magnitude; the relation holds at any speed, but the Newtonian p = m v does not near c
- Free particle: inside a potential the local wavelength varies
- The wavelength of a macroscopic body is defined but far below any measurable size
- The 0.5 % tolerance covers the rounding of a stated figure, it is not a physical uncertainty
## References
- de Broglie, L., Recherches sur la theorie des Quanta, Annales de Physique 10(3), 22-128 (1925), doi:10.1051/anphys/192510030022
- Griffiths, D. J. and Schroeter, D. F., Introduction to Quantum Mechanics, 3rd ed., Cambridge University Press (2018)
- CODATA 2022 recommended value of the Planck constant, https://physics.nist.gov/cgi-bin/cuu/Value?h
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit de_broglie under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
