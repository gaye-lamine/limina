# Design: law-nuclear_q
One JSON definition, no Rust code. Tests: `crates/limina-dsl/tests/quantum_nuclear.rs` (golden values, negative cases, properties) and `crates/limina-api/tests/library.rs` (engine-computed curves). The explorer plots the law through `POST /v1/curve`; the browser only draws. The claim unit must be convertible to `J`.
