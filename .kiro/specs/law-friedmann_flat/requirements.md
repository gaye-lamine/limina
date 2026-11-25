# Requirements: law-friedmann_flat
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/friedmann_flat.json`.
## Facts
- Name: Friedmann equation, flat universe. Kind: equality (predicted value, relative tolerance 0.005). Bounded quantity dimension: `Hz`.
- Equation: `sqrt(8*pi*G*rho/3)`
- Classification: `model`. Epistemic status: `model_bound`.
- Validity domain: Expansion rate H of a homogeneous, isotropic (FLRW) universe with zero spatial curvature (k = 0), H^2 = 8 pi G rho / 3, where rho is the total mass-energy density of all components divided by c^2, including dark energy treated as an effective density.
- Validity conditions: rho > 0
- Variables:
- `rho`: Total mass-energy density divided by c squared (kg/m^3)
## Limits of application
- Flat universe only: for k not zero the term -k c^2 / a^2 is not included, so the law does not apply
- rho is the total density of every component (matter, radiation, dark energy as an effective density); the value of a single component is not enough
- H and rho refer to the same epoch; the law does not give H today unless rho is the density today
- H is in s^-1 (unit Hz); a claim in km/s/Mpc must be converted first, that unit is not in the core units table
- A model of homogeneous cosmology, not a statement about local structure
## References
- Friedmann, A., Ueber die Kruemmung des Raumes, Zeitschrift fuer Physik 10, 377-386 (1922), doi:10.1007/BF01332580
- Ryden, B., Introduction to Cosmology, 2nd ed., Cambridge University Press
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim equal to the prediction is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim outside the tolerance band is IMPOSSIBLE with the wording "inconsistent with limit friedmann_flat under assumptions none". A non-positive density is OUTSIDE_MODEL_DOMAIN.
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
