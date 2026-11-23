//! Extraction errors. None carries the key or the raw model output.

use crate::provider::ProviderError;

/// Every way an extraction can fail.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ExtractError {
    /// The model answered, but not with a usable JSON object of the schema.
    /// The reason is fixed text, never the model output.
    #[error("extraction failed: {0}")]
    ExtractionFailed(&'static str),
    /// More than the allowed number of requests per minute.
    #[error("too many extraction requests; try again in a minute")]
    RateLimited,
    /// The provider could not be used.
    #[error(transparent)]
    Provider(#[from] ProviderError),
    /// No key is configured.
    #[error("extraction is not configured")]
    Unavailable,
}
