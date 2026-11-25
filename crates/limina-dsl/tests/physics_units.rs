//! D-25 reference tests: SI units and CODATA 2022 constants in DSL laws.
//! Every expected value is computed here from its textbook formula with the
//! constants written out, independently of the engine.
use std::collections::BTreeMap;

use limina_core::{
    ConfigurationSet, ConstraintsMap, Interval, Limit, ParameterEnclosure, ParameterId,
};
use limina_dsl::expr::Constant;
use limina_dsl::{DeclarativeLimit, DslError};
use serde_json::json;

fn law(
    direction: &str,
    bounded: &str,
    params: serde_json::Value,
    formula: &str,
) -> Result<DeclarativeLimit, DslError> {
    DeclarativeLimit::from_json(
        &json!({
            "id": "t", "title": "t", "direction": direction, "bounded_dimension": bounded,
            "parameters": params, "formula": formula, "epistemic_status": "theorem"
        })
        .to_string(),
    )
}

fn bound(limit: &DeclarativeLimit, values: &[(&str, f64)]) -> Interval {
    let map: BTreeMap<ParameterId, Interval> = values
        .iter()
        .map(|(n, v)| (ParameterId::new(*n), Interval::point(*v).expect("finite")))
        .collect();
    let set = ConfigurationSet::build(limit, &ConstraintsMap::new(), 8).expect("set");
    let config = set.iter().next().expect("one configuration");
    limit
        .bound(config, &ParameterEnclosure::new(map))
        .expect("bound")
}

fn param(name: &str, unit: &str) -> serde_json::Value {
    json!({"name": name, "dimension": unit, "units": [unit], "required": true, "test_range": [1, 10]})
}

#[test]
fn stefan_boltzmann_flux_in_w_per_m2() {
    let l = law(
        "max",
        "W/m^2",
        json!([{"name": "T", "kind": "absolute_temperature", "units": ["K"], "test_range": [10, 6000]}]),
        "sigma_SB*T^4",
    )
    .expect("dimensionally consistent");
    let b = bound(&l, &[("T", 5772.0)]);
    // sigma = 5.670374419e-8 W m^-2 K^-4 (CODATA 2022, exact).
    let expected = 5.670_374_419e-8 * 5772.0_f64.powi(4);
    assert_eq!(b.lo(), b.hi(), "sigma is exact");
    assert!((b.hi() - expected).abs() / expected < 1e-9);
}

#[test]
fn newtonian_gravity_carries_the_uncertainty_of_g() {
    let l = law(
        "max",
        "N",
        json!([param("m1", "kg"), param("m2", "kg"), param("r", "m")]),
        "G*m1*m2/r^2",
    )
    .expect("dimensionally consistent");
    let b = bound(&l, &[("m1", 5.972e24), ("m2", 1.0), ("r", 6.371e6)]);
    let mid = 6.674_30e-11 * 5.972e24 / 6.371e6_f64.powi(2);
    let half = 0.000_15e-11 * 5.972e24 / 6.371e6_f64.powi(2);
    assert!(
        b.lo() < mid && mid < b.hi(),
        "the value of G lies inside the bound range"
    );
    assert!(
        ((b.hi() - b.lo()) / 2.0 - half).abs() / half < 1e-6,
        "width is twice u(G)"
    );
}

#[test]
fn relativistic_and_quantum_laws_use_si_units() {
    let rest = law("max", "J", json!([param("m", "kg")]), "m*c^2").expect("E = mc^2");
    let b = bound(&rest, &[("m", 9.109_383_713_9e-31)]);
    let in_mev = b.hi() / (1.602_176_634e-19 * 1e6);
    // electron mass energy equivalent: 0.51099895069 MeV (CODATA 2022).
    assert!((in_mev - 0.510_998_950_69).abs() < 1e-9, "{in_mev}");
    let heisenberg = law("min", "kg m/s", json!([param("dx", "m")]), "hbar/(2*dx)")
        .expect("dp >= hbar / (2 dx)");
    let b = bound(&heisenberg, &[("dx", 1e-10)]);
    assert!((b.lo() - 1.054_571_817e-34 / 2e-10).abs() / b.lo() < 1e-9);
    let coulomb = law(
        "max",
        "N",
        json!([param("q1", "C"), param("q2", "C"), param("r", "m")]),
        "q1*q2/(4*pi*eps0*r^2)",
    )
    .expect("Coulomb");
    let b = bound(
        &coulomb,
        &[
            ("q1", 1.602_176_634e-19),
            ("q2", 1.602_176_634e-19),
            ("r", 5.291_772_105_44e-11),
        ],
    );
    // e^2 / (4 pi eps0 a0^2) = 8.2387e-8 N, the atomic unit of force.
    assert!(
        (b.hi() - 8.238_723_5e-8).abs() / 8.238_723_5e-8 < 1e-6,
        "{}",
        b.hi()
    );
}

#[test]
fn compound_and_prefixed_units_are_accepted_for_parameters() {
    let l = law(
        "max",
        "W",
        json!([
            {"name": "q", "dimension": "W/m^2", "units": ["W/m^2", "kW/m^2", "mW/cm^2"], "test_range": [1, 1000]},
            {"name": "A", "dimension": "m^2", "units": ["m^2", "cm^2"], "test_range": [1, 10]}
        ]),
        "q*A",
    );
    assert!(l.is_ok(), "{l:?}");
    let wrong = law(
        "max",
        "W",
        json!([{"name": "q", "dimension": "W/m^2", "units": ["W"], "test_range": [1, 2]}]),
        "q*1",
    );
    let err = wrong.expect_err("W is not W/m^2").to_string();
    assert!(err.contains("must be SI units of its dimension"), "{err}");
    let celsius = law(
        "max",
        "W",
        json!([{"name": "q", "dimension": "W", "units": ["°C"], "test_range": [1, 2]}]),
        "q",
    );
    assert!(celsius.is_err());
}

#[test]
fn the_dimension_checker_refuses_wrong_physics_with_the_new_constants() {
    // G m1 m2 / r is an energy, not a force.
    let err = law(
        "max",
        "N",
        json!([param("m1", "kg"), param("m2", "kg"), param("r", "m")]),
        "G*m1*m2/r",
    )
    .expect_err("dimension mismatch");
    assert!(matches!(err, DslError::DimensionMismatch { .. }), "{err:?}");
}

#[test]
fn the_constants_list_keeps_the_original_seven_first() {
    let all = Constant::all();
    let names: Vec<&str> = all.iter().map(|(n, _)| *n).collect();
    assert_eq!(&names[..7], ["kB", "h", "c", "e", "NA", "pi", "ln2"]);
    for n in [
        "hbar", "G", "eps0", "mu0", "m_e", "sigma_SB", "alpha_fs", "g_n",
    ] {
        assert!(names.contains(&n), "{n}");
    }
    assert_eq!(names.iter().filter(|n| **n == "c").count(), 1);
    assert_eq!(
        Constant::from_name("G").map(Constant::uncertainty),
        Some(0.000_15e-11)
    );
    assert_eq!(
        Constant::from_name("c").map(Constant::uncertainty),
        Some(0.0)
    );
}
