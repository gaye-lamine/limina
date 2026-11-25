# Design: law-lorentz_force
One JSON definition, no Rust code. Tests: `crates/limina-dsl/tests/lorentz.rs`. No vector type is added: the fields and the speed are magnitudes given as parameters, so `limina-core` and the DSL are unchanged. The claim unit must be convertible to `N`.
