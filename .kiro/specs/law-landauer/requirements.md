# Requirements: law-landauer
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/landauer.json`.
## Facts
- Name: Landauer limit. Kind: min bound. Bounded quantity dimension: `J`.
- Equation: `kB*T*ln2`
- Classification: `theoretical_bound`. Epistemic status: `theorem`.
- Validity domain: Heat dissipated into a thermal environment at absolute temperature T by the logically irreversible erasure of one bit of information.
- Validity conditions: T > 0
- Variables:
- `T`: Absolute temperature of the environment receiving the heat (K)
## Limits of application
- Applies to the erasure of a bit (a logically irreversible operation), not to the energy a whole computer draws
- Logically reversible computation has no such lower bound
- A lower bound on the heat per erased bit: real hardware dissipates many orders of magnitude more, and the bound says nothing about the energy per operation of a circuit
## References
- Landauer, R., Irreversibility and Heat Generation in the Computing Process, IBM Journal of Research and Development 5(3), 183-191 (1961), doi:10.1147/rd.53.0183
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound is IMPOSSIBLE with the wording "inconsistent with limit landauer under assumptions none".
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out (CODATA 2022), not by the DSL.
