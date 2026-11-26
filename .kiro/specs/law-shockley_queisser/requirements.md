# Requirements: law-shockley_queisser
Status: v1, frozen. Base: `limit-dsl` and `decision-engine` (unchanged). Definition: `data/limits/shockley_queisser.json`. Added after the 26/11 review of a multi-domain image.
## Facts
- Name: Shockley-Queisser limit. Kind: max bound. Bounded quantity dimension: `dimensionless`.
- Equation: `0.337`
- Option `cell_type = single_junction_one_sun`: `0.337`
- Option `cell_type = multi_junction_or_concentrated`: `0.868`
- Classification: `theoretical_bound`. Epistemic status: `model_bound`.
- Validity domain: Power conversion efficiency of a single-junction solar cell under unconcentrated sunlight (AM1.5G, cell at 300 K): at most 33.7 %, the detailed-balance maximum over all band gaps (reached near 1.34 eV). The limit of any particular material, silicon included, is lower or equal.
- Validity conditions: none in the DSL
## Limits of application
- Assumption cell_type: for a single-junction cell under unconcentrated sunlight the bound is 33.7 %; for multi-junction (tandem) cells or concentrated sunlight the bound used is 86.8 %, the detailed-balance limit of an infinite stack of junctions under maximum concentration (De Vos 1980).
- The maximum over band gaps is used, so the bound never exceeds what any single-junction material allows; it is not the lower limit of a specific material
- Hot-carrier, multiple-exciton and up/down-conversion schemes are outside its assumptions
## References
- Shockley, W. and Queisser, H. J., Detailed Balance Limit of Efficiency of p-n Junction Solar Cells, Journal of Applied Physics 32(3), 510-519 (1961), doi:10.1063/1.1736034
- Rühle, S., Tabulated values of the Shockley-Queisser limit for single junction solar cells, Solar Energy 130, 139-147 (2016), doi:10.1016/j.solener.2016.02.015
- De Vos, A., Detailed balance limit of the efficiency of tandem solar cells, Journal of Physics D: Applied Physics 13(5), 839-846 (1980), doi:10.1088/0022-3727/13/5/018
## Functional requirements
- **R-01** The law is embedded by `Registry::builtin` and served by `GET /v1/limits` with classification, references, validity domain and limitations.
- **R-02** A missing parameter gives INSUFFICIENT_DATA, never IMPOSSIBLE.
- **R-03** A claim at or inside the bound is never IMPOSSIBLE (property in `crates/limina-dsl/tests/review_laws.rs`, fixed seed 42, 256 cases, 1024 with CI set).
- **R-04** A claim beyond the bound in every configuration allowed by the text is IMPOSSIBLE with the wording "inconsistent with limit shockley_queisser under assumptions ..."; when an assumption is not stated and one option allows the claim, the verdict is CONDITIONAL.
- **R-05** Expected values in the tests are computed from the textbook formula with the constants written out, not by the DSL.
- **R-06** The built-in matcher (`crates/limina-extract/src/matcher.rs`) chooses the law by keyword and dimension; the model's limit id is a hint only.
