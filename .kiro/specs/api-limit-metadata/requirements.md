# Requirements: api-limit-metadata
Status: v1, frozen. Amends `api-evaluate` v1 (additive). Adds one function to `limina-core` (decision-engine unit table, design 3.4).

## Requirements
- R-01 `GET /v1/limits` returns for each limit a field `claim_unit`: the canonical unit symbol of the bounded quantity. It is `dimensionless` for `betz` and `carnot`.
- R-02 Each parameter of `GET /v1/limits` has a field `units`: the allowed unit symbols for that parameter, derived from its kind and from the core unit table. For `AbsoluteTemperature` the list is `["K"]`.
- R-03 Fields are only added. Existing fields and the `POST /v1/evaluate` contract do not change. Existing API tests keep passing, with one exception: the exact-JSON assertion on Carnot `parameters` in `tests/limits.rs` now includes `units` (strengthened, not weakened).
- R-04 `limina-core` exposes `units_for_dimension(Dimension) -> Vec<&'static str>`, the table symbols of that dimension in table order. `limina-api` keeps no unit list of its own.
- R-05 Tests: exact expected `claim_unit` and `units` for betz and carnot; every listed unit is accepted by `POST /v1/evaluate`; `units_for_dimension` matches the table for each dimension.

## Known limit
`K` is listed for temperature parameters but `Quantity::from_unit` rejects it (`TemperatureKindRequired`); the API accepts it through the temperature types (api-evaluate R-10). The core test treats `K` accordingly.
