# Design: api-limit-metadata
Status: v1, frozen.

- `limina-core/src/units.rs`: `pub fn units_for_dimension(Dimension)` filters `UNIT_TABLE` by dimension. Re-exported from `lib.rs`. No existing item changes.
- `limina-api/src/dto.rs`: `LimitDto.claim_unit: String`, `ParameterSpecDto.units: Vec<String>`.
- `limina-api/src/registry.rs`: `units_of(kind)` maps `AbsoluteTemperature` and `TemperatureDifference` to the temperature dimension, and `Plain(d)` to `d`, then calls `units_for_dimension`. `claim_unit` is the first unit of `units_of(limit.bounded_kind())`. If a dimension has no unit, `claim_unit` is an empty string (not the case for betz and carnot).
- README: example of `GET /v1/limits` updated.
