//! Graph endpoints computed from the law by the engine: the surface
//! `POST /v1/surface`, and the helpers of `POST /v1/curve` for uncertainty
//! bands and validity zones. No language model, no verdict.
use std::collections::BTreeMap;

use axum::body::Bytes;
use axum::extract::State;
use axum::Json;
use limina_core::{AssumptionId, ConstraintsMap, Direction, EngineError, Limit, OptionId};
use limina_dsl::{
    bound_surface, decisive, zones, CurveError, CurvePoint, EqualityLimit, SurfaceAxis,
};
use limina_extract::matcher::{CONVERSION_PROCESS, HEAT_ENGINE_CYCLE};

use crate::authoring::si;
use crate::dto::{CurvePointDto, CurveRequest, ParameterDto, SurfaceRequest, SurfaceResponse};
use crate::error::ApiError;
use crate::pipeline::default_range;
use crate::{registry, AppState};

/// Default number of samples per axis of `POST /v1/surface`.
pub const DEFAULT_SURFACE_SAMPLES: usize = 30;

/// The constraints of a request as engine constraints.
pub(crate) fn constraints_of(map: &BTreeMap<String, Vec<String>>) -> ConstraintsMap {
    let mut constraints = ConstraintsMap::new();
    for (a, options) in map {
        constraints.allow(
            AssumptionId::new(a),
            options.iter().map(OptionId::new).collect(),
        );
    }
    constraints
}

/// The constraints of a plot. A plot shows the law's bound where it
/// applies: Carnot, when the request leaves its conversion process open, is
/// drawn for a heat engine cycle (the other option bounds nothing). Plots
/// carry no verdict.
pub(crate) fn plot_constraints(id: &str, map: &BTreeMap<String, Vec<String>>) -> ConstraintsMap {
    let mut map = map.clone();
    if id == "carnot" {
        map.entry(CONVERSION_PROCESS.to_string())
            .or_insert_with(|| vec![HEAT_ENGINE_CYCLE.to_string()]);
    }
    constraints_of(&map)
}

/// The given parameters in SI, except the free ones; a name the limit does
/// not declare is refused.
pub(crate) fn fixed_si(
    limit: &dyn Limit,
    parameters: &BTreeMap<String, ParameterDto>,
    free: &[&str],
) -> Result<BTreeMap<String, f64>, ApiError> {
    let mut fixed = BTreeMap::new();
    for (name, p) in parameters {
        if !limit.parameters().iter().any(|s| s.id.as_str() == name) {
            return Err(EngineError::UnknownParameter(name.clone()).into());
        }
        if !free.contains(&name.as_str()) {
            fixed.insert(name.clone(), si(p.value, &p.unit)?);
        }
    }
    Ok(fixed)
}

/// The fixed parameters as SI enclosures: `value -/+ uncertainty` in the
/// unit of the value, converted, ordered. Parameters without uncertainty
/// stay points.
pub(crate) fn enclosed_si(
    limit: &dyn Limit,
    request: &CurveRequest,
    fixed: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, (f64, f64)>, ApiError> {
    for (name, u) in &request.uncertainties {
        if !limit.parameters().iter().any(|s| s.id.as_str() == name) {
            return Err(EngineError::UnknownParameter(name.clone()).into());
        }
        if !(u.is_finite() && *u >= 0.0) {
            return Err(ApiError::InvalidCurveRequest(format!(
                "uncertainty of {name} must be finite and not negative"
            )));
        }
    }
    let mut out = BTreeMap::new();
    for (name, v) in fixed {
        let enclosure = match (
            request.uncertainties.get(name),
            request.parameters.get(name),
        ) {
            (Some(u), Some(p)) => {
                let a = si(p.value - u, &p.unit)?;
                let b = si(p.value + u, &p.unit)?;
                (a.min(b), a.max(b))
            }
            _ => (*v, *v),
        };
        out.insert(name.clone(), enclosure);
    }
    Ok(out)
}

/// Curve points as served, with the deciding bound for `direction`.
pub(crate) fn dto_points(direction: Direction, points: &[CurvePoint]) -> Vec<CurvePointDto> {
    points
        .iter()
        .map(|p| CurvePointDto {
            x: p.x,
            b_min: p.b_min,
            b_max: p.b_max,
            decisive: decisive(direction, p),
        })
        .collect()
}

/// Validity zones of an equality law: where a validity condition fails for
/// the fixed parameters and the free one at the sample.
pub(crate) fn equality_zones(
    law: &EqualityLimit,
    fixed: &BTreeMap<String, f64>,
    free: &str,
    points: &[CurvePoint],
) -> Vec<(f64, f64)> {
    zones(points, |p| {
        let mut values = fixed.clone();
        values.insert(free.to_string(), p.x);
        law.outside_domain(&values).is_some()
    })
}

pub(crate) fn curve_error(e: CurveError) -> ApiError {
    match e {
        CurveError::UnknownFreeParameter(p) => ApiError::UnknownFreeParameter(p),
        CurveError::MissingFixedParameter(p) => ApiError::MissingFixedParameter(p),
        CurveError::Invalid(m) => ApiError::InvalidCurveRequest(m),
        CurveError::Engine(e) => ApiError::Engine(e),
    }
}

/// `POST /v1/surface`: the bound against two parameters, computed by the
/// limit itself, for the heat map of the explorer. No verdict.
pub async fn surface(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<SurfaceResponse>, ApiError> {
    let request: SurfaceRequest =
        serde_json::from_slice(&body).map_err(|e| ApiError::InvalidJson(e.to_string()))?;
    Ok(Json(surface_with(&state, &request)?))
}

/// The surface itself, shared by the endpoint and its tests.
pub fn surface_with(
    state: &AppState,
    request: &SurfaceRequest,
) -> Result<SurfaceResponse, ApiError> {
    let id = request.limit_id.as_str();
    let limit: Box<dyn Limit> = match registry::find_limit(id) {
        Some(l) => l,
        None => Box::new(
            state
                .authored(id)
                .ok_or_else(|| ApiError::UnknownLimit(id.to_string()))?,
        ),
    };
    let samples = request.samples.unwrap_or(DEFAULT_SURFACE_SAMPLES);
    let axis = |name: &str, range: Option<[f64; 2]>| -> Result<SurfaceAxis, ApiError> {
        let range = match range {
            Some([lo, hi]) => (lo, hi),
            None => default_range(state, limit.as_ref(), id, name)
                .ok_or_else(|| ApiError::NoSearchRange(name.to_string()))?,
        };
        Ok(SurfaceAxis {
            name: name.to_string(),
            range,
            samples,
        })
    };
    let x = axis(&request.x_parameter, request.x_range)?;
    let y = axis(&request.y_parameter, request.y_range)?;
    let fixed = fixed_si(
        limit.as_ref(),
        &request.parameters,
        &[&request.x_parameter, &request.y_parameter],
    )?;
    let values = bound_surface(
        limit.as_ref(),
        &plot_constraints(id, &request.constraints),
        &fixed,
        &x,
        &y,
    )
    .map_err(curve_error)?;
    Ok(SurfaceResponse {
        limit_id: id.to_string(),
        direction: match limit.direction() {
            Direction::Max => "Max",
            Direction::Min => "Min",
        },
        x_parameter: x.name.clone(),
        y_parameter: y.name.clone(),
        x: x.values(),
        y: y.values(),
        values,
    })
}
