# Design: law-shannon_hartley
One JSON definition, no Rust code. Tests: `crates/limina-dsl/tests/thermo_info.rs` (golden values, negative cases, properties) and `crates/limina-api/tests/library.rs` (listing, evaluation, analysis). The unit of the claim is the declared unit `bit/s`; a claim in another unit is refused by the extractor, not converted.
