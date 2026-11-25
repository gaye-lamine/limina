//! API errors and their HTTP mapping.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use limina_core::EngineError;
use limina_extract::ExtractError;
use serde::Serialize;

/// Every error the API returns.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ApiError {
    /// The body is not valid JSON or does not match the request schema.
    #[error("invalid request body: {0}")]
    InvalidJson(String),

    /// `limit_id` names no limit served by this API.
    #[error("unknown limit: {0}")]
    UnknownLimit(String),

    /// The engine refused the input or the configuration.
    #[error(transparent)]
    Engine(#[from] EngineError),
    /// `POST /v1/analyze` without a configured key.
    #[error("extraction is not available: NVIDIA_API_KEY is not set")]
    ExtractionUnavailable,
    /// The model answer was not a usable JSON object of the schema. The
    /// message is fixed text, never the model output.
    #[error("extraction failed: {0}")]
    ExtractionFailed(&'static str),
    /// The provider could not be used. Never the body, the headers or the key.
    #[error("{0}")]
    ExtractionProvider(String),
    /// More extraction requests than allowed per minute.
    #[error("too many extraction requests; try again in a minute")]
    RateLimited,
    /// The text to analyse is longer than allowed.
    #[error("the text is longer than {max} characters")]
    TextTooLong {
        /// Largest accepted length, in characters.
        max: usize,
    },
    /// The text to analyse is empty.
    #[error("the text is empty")]
    EmptyText,
    /// The model gave no valid limit definition after every repair.
    #[error("no valid definition after {attempts} attempts; last error: {last_error}")]
    LimitNotAuthored {
        /// Answers tried.
        attempts: usize,
        /// The last error sent back to the model.
        last_error: String,
    },
    /// `POST /v1/solve`: the bound is not monotone in the free parameter.
    #[error("the bound is not monotone in {parameter} on the search range ({monotonicity}): no threshold is given")]
    NotMonotone {
        /// The free parameter.
        parameter: String,
        /// What the sampling found.
        monotonicity: String,
    },
    /// `POST /v1/solve`: no plausible domain and no test range.
    #[error(
        "no search range for {0}: give a range, or declare a plausible domain or a test range"
    )]
    NoSearchRange(String),
    /// `POST /v1/solve`: the bound never equals the claim on the range.
    #[error("the bound does not reach the claim for {parameter} in [{lo}, {hi}]")]
    NoThresholdInRange {
        /// The free parameter.
        parameter: String,
        /// Lower end of the range.
        lo: f64,
        /// Upper end of the range.
        hi: f64,
    },
    /// `POST /v1/solve`: the free parameter is not declared by the limit.
    #[error("the limit has no parameter {0}")]
    UnknownFreeParameter(String),
    /// `POST /v1/solve`: a declared parameter has no fixed value.
    #[error("parameter {0} needs a fixed value")]
    MissingFixedParameter(String),
    /// `POST /v1/curve`: the range or the point count is invalid.
    #[error("invalid curve request: {0}")]
    InvalidCurveRequest(String),
}

impl From<ExtractError> for ApiError {
    fn from(e: ExtractError) -> ApiError {
        match e {
            ExtractError::ExtractionFailed(reason) => ApiError::ExtractionFailed(reason),
            ExtractError::RateLimited => ApiError::RateLimited,
            ExtractError::Provider(p) => ApiError::ExtractionProvider(p.to_string()),
            ExtractError::Unavailable => ApiError::ExtractionUnavailable,
        }
    }
}

/// Body of an error response: `{"error": {"code", "message"}}`.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// The error.
    pub error: ErrorDetail,
}

/// Stable code and human-readable message of an error.
#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    /// Stable code, the name of the error variant.
    pub code: &'static str,
    /// Message for a human; may change between versions.
    pub message: String,
}

impl ApiError {
    /// Stable error code.
    pub fn code(&self) -> &'static str {
        match self {
            ApiError::InvalidJson(_) => "InvalidJson",
            ApiError::UnknownLimit(_) => "UnknownLimit",
            ApiError::Engine(e) => engine_code(e),
            ApiError::ExtractionUnavailable => "ExtractionUnavailable",
            ApiError::ExtractionFailed(_) => "ExtractionFailed",
            ApiError::ExtractionProvider(_) => "ExtractionProviderError",
            ApiError::RateLimited => "RateLimited",
            ApiError::TextTooLong { .. } => "TextTooLong",
            ApiError::EmptyText => "EmptyText",
            ApiError::LimitNotAuthored { .. } => "LimitNotAuthored",
            ApiError::NotMonotone { .. } => "NotMonotone",
            ApiError::NoSearchRange(_) => "NoSearchRange",
            ApiError::NoThresholdInRange { .. } => "NoThresholdInRange",
            ApiError::UnknownFreeParameter(_) => "UnknownFreeParameter",
            ApiError::MissingFixedParameter(_) => "MissingFixedParameter",
            ApiError::InvalidCurveRequest(_) => "InvalidCurveRequest",
        }
    }

    /// HTTP status: 400 for a malformed body, 503 without extraction, 502
    /// when the model or its provider fails, 429 when rate limited, 422 for
    /// everything else.
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::InvalidJson(_) => StatusCode::BAD_REQUEST,
            ApiError::ExtractionUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::ExtractionFailed(_) | ApiError::ExtractionProvider(_) => {
                StatusCode::BAD_GATEWAY
            }
            ApiError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiError::UnknownLimit(_)
            | ApiError::Engine(_)
            | ApiError::TextTooLong { .. }
            | ApiError::EmptyText
            | ApiError::LimitNotAuthored { .. }
            | ApiError::NotMonotone { .. }
            | ApiError::NoSearchRange(_)
            | ApiError::NoThresholdInRange { .. }
            | ApiError::UnknownFreeParameter(_)
            | ApiError::MissingFixedParameter(_)
            | ApiError::InvalidCurveRequest(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

/// Stable code of an engine error. No wildcard arm: a new `EngineError`
/// variant does not compile until it has a code.
fn engine_code(error: &EngineError) -> &'static str {
    match error {
        EngineError::IncompatibleDimension { .. } => "IncompatibleDimension",
        EngineError::TemperatureKindMismatch => "TemperatureKindMismatch",
        EngineError::TemperatureKindRequired => "TemperatureKindRequired",
        EngineError::UnknownUnit(_) => "UnknownUnit",
        EngineError::NegativeAbsoluteTemperature(_) => "NegativeAbsoluteTemperature",
        EngineError::NonFiniteValue { .. } => "NonFiniteValue",
        EngineError::InvertedInterval => "InvertedInterval",
        EngineError::DivisionByIntervalContainingZero => "DivisionByIntervalContainingZero",
        EngineError::InvalidCoverageFactor => "InvalidCoverageFactor",
        EngineError::InvalidGuard => "InvalidGuard",
        EngineError::InvalidMaxConfigurations => "InvalidMaxConfigurations",
        EngineError::NegativeUncertainty => "NegativeUncertainty",
        EngineError::InvalidBound { .. } => "InvalidBound",
        EngineError::UnknownAssumption(_) => "UnknownAssumption",
        EngineError::ConstraintOutsideDomain { .. } => "ConstraintOutsideDomain",
        EngineError::UnknownParameter(_) => "UnknownParameter",
        EngineError::EmptyConfigurationSet => "EmptyConfigurationSet",
        EngineError::ConfigurationSetTooLarge { .. } => "ConfigurationSetTooLarge",
        EngineError::NotApplicable { .. } => "NotApplicable",
        EngineError::InvalidTemperatureOrder { .. } => "InvalidTemperatureOrder",
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code(),
                message: self.to_string(),
            },
        };
        (self.status(), Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_errors_map_to_422_with_the_variant_name() {
        let err = ApiError::from(EngineError::NotApplicable {
            limit_id: "betz".to_string(),
            assumption: "reference_area".to_string(),
            option: "shrouded_rotor_area_only".to_string(),
        });
        assert_eq!(err.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.code(), "NotApplicable");
        assert!(err.to_string().contains("not applicable"));
    }

    #[test]
    fn invalid_json_is_400() {
        let err = ApiError::InvalidJson("expected value".to_string());
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.code(), "InvalidJson");
    }

    #[test]
    fn unknown_limit_is_422() {
        let err = ApiError::UnknownLimit("x".to_string());
        assert_eq!(err.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.code(), "UnknownLimit");
    }

    #[test]
    fn invalid_temperature_order_keeps_its_code() {
        let err = ApiError::from(EngineError::InvalidTemperatureOrder {
            limit_id: "carnot".to_string(),
        });
        assert_eq!(err.code(), "InvalidTemperatureOrder");
        assert_eq!(err.to_string(), "limit carnot requires Th > Tc > 0");
    }
}
