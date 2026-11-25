# Requirements: law-abbe_limit
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/abbe_limit.json`.
## Facts
- Name: Abbe diffraction limit. Kind: min bound. Bounded quantity dimension: `m`.
- Equation: `wavelength/(2*aperture)`
- Classification: `theoretical_bound`. Epistemic status: `model_bound`.
- Validity domain: Smallest resolvable period d = lambda / (2 NA) of a far-field optical microscope with a lens of numerical aperture NA, at vacuum wavelength lambda, with conventional (linear, non-saturated) imaging.
- Validity conditions: none checked by the core
- Variables:
- `wavelength`: Vacuum wavelength of the light (m, um, nm)
- `aperture`: Numerical aperture of the objective (dimensionless)
## Limits of application
- Far-field imaging with linear optics; near-field microscopy, STED, localisation microscopy (PALM, STORM) and structured illumination beat it, because they do not satisfy these assumptions
- A claim of localisation precision of a single emitter is not a resolution in this sense
- aperture is the numerical aperture NA = n sin(theta) of the objective, wavelength is the vacuum wavelength
- Defined for a positive numerical aperture; the core checks no condition on it here
- A lower bound on the resolvable distance: real instruments resolve a larger distance
## References
- Abbe, E., Beitraege zur Theorie des Mikroskops und der mikroskopischen Wahrnehmung, Archiv fuer mikroskopische Anatomie 9(1), 413-468 (1873), doi:10.1007/BF02956173
- Born, M. and Wolf, E., Principles of Optics, 7th ed., Cambridge University Press (1999)
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound is IMPOSSIBLE with the wording "inconsistent with limit abbe_limit under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
