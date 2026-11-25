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
- **R-04** (amended v1.1) Assumption `imaging_method`: `conventional_far_field` (bound lambda / (2 NA)) or `super_resolution` (STED, localisation, structured illumination, near-field: this law gives no bound, 0). A claim below the bound with the method stated as `conventional_far_field` is IMPOSSIBLE, worded "inconsistent with limit abbe_limit under assumptions imaging method = conventional far-field imaging". With the method not stated it is CONDITIONAL, "possible only if imaging method = super-resolution techniques". Reference added: Hell and Wichmann, Optics Letters 19(11), 780-782 (1994), doi:10.1364/OL.19.000780.
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
