# Requirements: law-escape_speed_min
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/escape_speed_min.json`. Added after the 26/11 review of a multi-domain image.
## Facts
- Name: Minimum speed to escape a body's gravity. Kind: min bound. Bounded quantity dimension: `m/s`.
- Equation: `sqrt(2*GM/r)`
- Option `propulsion = ballistic`: `sqrt(2*GM/r)`
- Option `propulsion = powered`: `0*sqrt(2*GM/r)`
- Classification: `theoretical_bound`. Epistemic status: `theorem`.
- Validity domain: A body launched at distance r from the centre of a spherically symmetric mass with gravitational parameter GM escapes to infinity only if its launch speed is at least sqrt(2 GM / r), when no further propulsion acts on it (Newtonian gravity, energy conservation).
- Validity conditions: none in the DSL
- Variables:
- `GM`: Gravitational parameter of the body (G times its mass) (m^3/s^2)
- `r`: Distance from the body's centre at launch (m, km)
## Limits of application
- Assumption propulsion: under ballistic (no further propulsion) the launch speed must be at least sqrt(2 GM / r); under powered flight this law gives no bound (the bound used is 0).
- Speed in the inertial frame of the body's centre: a launch along the rotation of the body gains its surface speed (about 0.46 km/s at Earth's equator)
- Atmospheric drag and the pull of other bodies are ignored; drag only raises the speed needed
- Newtonian: not valid near a black hole
## References
- Kleppner, D. and Kolenkow, R. J., An Introduction to Mechanics, 2nd ed., Cambridge University Press (2014)
- IAU 2015 Resolution B3 on recommended nominal conversion constants for selected solar and planetary properties, arXiv:1510.07674
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property in `crates/limina-dsl/tests/review_laws.rs`, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound in every configuration allowed by the text is IMPOSSIBLE with the wording "inconsistent with limit escape_speed_min under assumptions ..."; when an assumption is not stated and one option allows the claim, the verdict is CONDITIONAL.
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out, not by the DSL.
- **R-06** The built-in matcher (`crates/limina-extract/src/matcher.rs`) chooses the law by keyword and dimension; the model's limit id is a hint only.
