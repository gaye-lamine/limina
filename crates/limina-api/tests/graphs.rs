//! Graph data computed by the engine: uncertainty bands, validity zones,
//! equality curves and surfaces.
use std::collections::BTreeMap;

use limina_api::dto::{CurveRequest, ParameterDto, SurfaceRequest};
use limina_api::error::ApiError;
use limina_api::graphs::surface_with;
use limina_api::pipeline::curve_with;
use limina_api::AppState;

const C: f64 = 299_792_458.0;

fn param(value: f64, unit: &str) -> ParameterDto {
    ParameterDto {
        value,
        unit: unit.to_string(),
    }
}

fn carnot_curve(range: [f64; 2], points: usize) -> CurveRequest {
    CurveRequest {
        limit_id: "carnot".to_string(),
        constraints: BTreeMap::new(),
        parameters: BTreeMap::from([("Tc".to_string(), param(300.0, "K"))]),
        free_parameter: "Th".to_string(),
        range: Some(range),
        points: Some(points),
        uncertainties: BTreeMap::new(),
    }
}

fn kinetic_curve(range: [f64; 2], points: usize) -> CurveRequest {
    CurveRequest {
        limit_id: "relativistic_kinetic_energy".to_string(),
        constraints: BTreeMap::new(),
        parameters: BTreeMap::from([("m".to_string(), param(1.0, "kg"))]),
        free_parameter: "v".to_string(),
        range: Some(range),
        points: Some(points),
        uncertainties: BTreeMap::new(),
    }
}

#[test]
fn a_bound_curve_has_no_band_and_no_lower_edge_unless_asked() {
    let r = curve_with(&AppState::default(), &carnot_curve([310.0, 400.0], 10)).expect("curve");
    assert_eq!(r.kind, "bound");
    assert!(r.band.is_none() && r.lower.is_none());
    assert!(r.validity_zones.is_empty());
}

#[test]
fn the_uncertainty_band_encloses_the_nominal_curve() {
    let mut request = carnot_curve([350.0, 400.0], 6);
    request.uncertainties.insert("Tc".to_string(), 5.0);
    let r = curve_with(&AppState::default(), &request).expect("curve");
    let band = r.band.expect("band");
    assert_eq!(band.len(), r.points.len());
    for (n, b) in r.points.iter().zip(&band) {
        assert!(b.b_max.expect("b") >= n.b_max.expect("n") - 1e-12);
        assert!(b.b_min.expect("b") <= n.b_min.expect("n") + 1e-12);
    }
    // At Th = 400 K: 1 - 295/400 from Tc - 5 K, from the formula.
    let last = band.last().expect("point");
    assert!((last.b_max.expect("b") - (1.0 - 295.0 / 400.0)).abs() < 1e-9);
    assert!((last.b_min.expect("b") - (1.0 - 305.0 / 400.0)).abs() < 1e-9);
}

#[test]
fn the_validity_zone_of_a_bound_law_is_where_it_is_undefined() {
    let r = curve_with(&AppState::default(), &carnot_curve([200.0, 400.0], 101)).expect("curve");
    assert_eq!(r.validity_zones.len(), 1);
    let [from, to] = r.validity_zones[0];
    assert_eq!(from, 200.0);
    assert!((290.0..=300.0).contains(&to), "{to}");
}

#[test]
fn an_equality_curve_has_a_tolerance_band_around_the_prediction() {
    let r = curve_with(&AppState::default(), &kinetic_curve([1.0e7, 2.0e8], 5)).expect("curve");
    assert_eq!(r.kind, "equality");
    let lower = r.lower.expect("lower edge");
    for (up, low) in r.points.iter().zip(&lower) {
        let v = up.x;
        let predicted = C * C * (1.0 / (1.0 - (v / C) * (v / C)).sqrt() - 1.0);
        let (u, l) = (up.decisive.expect("u"), low.decisive.expect("l"));
        assert!((u - predicted * 1.005).abs() <= predicted * 1e-9, "{v}");
        assert!((l - predicted * 0.995).abs() <= predicted * 1e-9, "{v}");
    }
    assert!(r.validity_zones.is_empty());
}

#[test]
fn an_equality_curve_reports_where_the_speed_reaches_c() {
    let r = curve_with(&AppState::default(), &kinetic_curve([2.0e8, 4.0e8], 21)).expect("curve");
    assert_eq!(r.validity_zones.len(), 1);
    let [from, to] = r.validity_zones[0];
    assert!((C..C + 1.0e7).contains(&from), "{from}");
    assert_eq!(to, 4.0e8);
}

#[test]
fn a_negative_uncertainty_is_refused() {
    let mut request = carnot_curve([350.0, 400.0], 6);
    request.uncertainties.insert("Tc".to_string(), -1.0);
    let err = curve_with(&AppState::default(), &request).expect_err("refused");
    assert_eq!(err.code(), "InvalidCurveRequest");
}

fn carnot_surface(samples: usize) -> SurfaceRequest {
    SurfaceRequest {
        limit_id: "carnot".to_string(),
        constraints: BTreeMap::new(),
        parameters: BTreeMap::new(),
        x_parameter: "Th".to_string(),
        y_parameter: "Tc".to_string(),
        x_range: Some([250.0, 450.0]),
        y_range: Some([100.0, 300.0]),
        samples: Some(samples),
    }
}

#[test]
fn the_surface_is_the_bound_on_the_grid() {
    let r = surface_with(&AppState::default(), &carnot_surface(5)).expect("surface");
    assert_eq!(r.x, vec![250.0, 300.0, 350.0, 400.0, 450.0]);
    assert_eq!(r.y, vec![100.0, 150.0, 200.0, 250.0, 300.0]);
    assert_eq!(r.direction, "Max");
    for (j, row) in r.values.iter().enumerate() {
        for (i, v) in row.iter().enumerate() {
            let (th, tc) = (r.x[i], r.y[j]);
            match v {
                Some(b) => {
                    assert!(th > tc, "{th} {tc}");
                    assert!((b - (1.0 - tc / th)).abs() < 1e-9, "{th} {tc}");
                }
                None => assert!(th <= tc, "{th} {tc}"),
            }
        }
    }
    assert!(r.values[4][0].is_none() && r.values[0][0].is_some());
}

#[test]
fn the_surface_grid_is_capped_and_needs_two_distinct_parameters() {
    let state = AppState::default();
    let err = surface_with(&state, &carnot_surface(51)).expect_err("too large");
    assert_eq!(err.code(), "InvalidCurveRequest");
    let mut same = carnot_surface(5);
    same.y_parameter = "Th".to_string();
    assert!(matches!(
        surface_with(&state, &same),
        Err(ApiError::InvalidCurveRequest(_))
    ));
    let mut unknown = carnot_surface(5);
    unknown.limit_id = "nothing".to_string();
    assert_eq!(
        surface_with(&state, &unknown).expect_err("unknown").code(),
        "UnknownLimit"
    );
}
