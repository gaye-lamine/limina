# Design: law-faraday_emf
One JSON definition, no Rust code. Tests: `crates/limina-dsl/tests/em_optics.rs` (golden values, negative cases, properties) and `crates/limina-api/tests/library.rs` (listing; engine-computed curves for coulomb and abbe_limit). The explorer plots the law through `POST /v1/curve`; the browser only draws. The claim unit must be convertible to `V`.
