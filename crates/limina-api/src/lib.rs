//! Limina HTTP API over the deterministic decision engine.
//!
//! The verdict is computed only by `limina-core`. This crate converts JSON to
//! engine inputs and engine outputs to JSON. `POST /v1/analyze` uses
//! `limina-extract` to read claims out of a text; the language model only
//! reads numbers and every verdict still comes from the engine.
//! `POST /v1/evaluate` never calls a language model.

pub mod analyze;
pub mod authoring;
pub mod convert;
pub mod dto;
pub mod error;
pub mod graphs;
pub mod handlers;
pub mod labels;
pub mod pipeline;
pub mod registry;
pub mod response;

use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use limina_dsl::DeclarativeLimit;

use axum::routing::{get, post};
use axum::Router;
use limina_extract::{
    ExtractConfig, ExtractError, ExtractionProvider, Extractor, OpenAiProvider, VisionProvider,
};
use std::collections::BTreeMap;

/// Shared state: the extractor, present only when a key is configured, and
/// the limits authored while the server runs (in memory, never persisted).
#[derive(Clone, Default)]
pub struct AppState {
    extractor: Option<Arc<Extractor>>,
    authored: Arc<RwLock<Vec<DeclarativeLimit>>>,
    second: Option<Arc<dyn ExtractionProvider>>,
    vision: Option<Arc<dyn VisionProvider>>,
    agreements: Arc<RwLock<BTreeMap<String, String>>>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("extraction", &self.extraction_available())
            .field(
                "model",
                &self.extractor.as_ref().map(|e| e.model().to_string()),
            )
            .finish()
    }
}

impl AppState {
    /// Extraction through the OpenAI-compatible provider when the
    /// configuration has a key; no extraction otherwise.
    pub fn from_config(config: &ExtractConfig) -> Result<AppState, ExtractError> {
        if !config.has_key() {
            return Ok(AppState::default());
        }
        let second = OpenAiProvider::new(&config.with_model(config.second_model()))?;
        let vision = OpenAiProvider::new(&config.with_model(config.vision_model()))?;
        Ok(
            AppState::with_provider(Arc::new(OpenAiProvider::new(config)?))
                .with_second_model(Arc::new(second))
                .with_vision(Arc::new(vision)),
        )
    }

    /// Enables two-model law authoring in `POST /v1/analyze`, with this
    /// provider as model B. Without it, a claim with no built-in limit is
    /// reported as such and nothing is authored.
    pub fn with_second_model(mut self, provider: Arc<dyn ExtractionProvider>) -> AppState {
        self.second = Some(provider);
        self
    }

    /// Enables `POST /v1/transcribe` with this vision model.
    pub fn with_vision(mut self, vision: Arc<dyn VisionProvider>) -> AppState {
        self.vision = Some(vision);
        self
    }

    pub(crate) fn second(&self) -> Option<&dyn ExtractionProvider> {
        self.second.as_deref()
    }

    pub(crate) fn vision(&self) -> Option<&dyn VisionProvider> {
        self.vision.as_deref()
    }

    /// Records the agreement status of an authored limit.
    pub fn set_agreement(&self, id: &str, status: &str) {
        self.agreements
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id.to_string(), status.to_string());
    }

    /// The agreement status of an authored limit, when it has one.
    pub fn agreement(&self, id: &str) -> Option<String> {
        self.agreements
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)
            .cloned()
    }

    /// What the pipeline can do: `GET /v1/pipeline`.
    pub fn pipeline_info(&self) -> dto::PipelineInfoDto {
        let mut models = Vec::new();
        if let Some(e) = self.extractor() {
            models.push(e.model().to_string());
        }
        if let Some(s) = self.second() {
            models.push(s.model().to_string());
        }
        if let Some(v) = self.vision() {
            models.push(v.model().to_string());
        }
        dto::PipelineInfoDto {
            extraction: self.extraction_available(),
            law_authoring: self.extraction_available() && self.second.is_some(),
            vision: self.vision.is_some(),
            models,
        }
    }

    /// [`AppState::from_config`] on the process environment.
    pub fn from_env() -> Result<AppState, ExtractError> {
        AppState::from_config(&ExtractConfig::from_env())
    }

    /// Extraction through any provider, on the served limits.
    pub fn with_provider(provider: Arc<dyn ExtractionProvider>) -> AppState {
        AppState::with_extractor(Extractor::new(provider, registry::catalog()))
    }

    /// Extraction through this extractor.
    pub fn with_extractor(extractor: Extractor) -> AppState {
        AppState {
            extractor: Some(Arc::new(extractor)),
            ..AppState::default()
        }
    }

    /// Whether `POST /v1/analyze` can run.
    pub fn extraction_available(&self) -> bool {
        self.extractor.is_some()
    }

    pub(crate) fn extractor(&self) -> Option<&Extractor> {
        self.extractor.as_deref()
    }

    /// Adds an authored limit, replacing one with the same id.
    pub fn register(&self, limit: DeclarativeLimit) {
        let mut all = self.authored.write().unwrap_or_else(|p| p.into_inner());
        all.retain(|l| l.definition().id != limit.definition().id);
        all.push(limit);
    }

    /// The authored limit with this id.
    pub fn authored(&self, id: &str) -> Option<DeclarativeLimit> {
        let all = self.authored.read().unwrap_or_else(|p| p.into_inner());
        all.iter().find(|l| l.definition().id == id).cloned()
    }

    /// Every authored limit, in registration order.
    pub fn authored_limits(&self) -> Vec<DeclarativeLimit> {
        self.authored
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

/// The line printed at startup. It names the model, never the key.
pub fn startup_message(addr: SocketAddr, state: &AppState) -> String {
    match state.extractor() {
        Some(e) => format!(
            "limina-api listening on http://{addr}; extraction enabled (model {})",
            e.model()
        ),
        None => format!(
            "limina-api listening on http://{addr}; extraction disabled (NVIDIA_API_KEY not set)"
        ),
    }
}

/// The router without extraction.
pub fn router() -> Router {
    router_with(AppState::default())
}

/// The router with every endpoint of the API.
pub fn router_with(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/v1/limits", get(handlers::limits))
        .route("/v1/evaluate", post(handlers::evaluate))
        .route("/v1/capabilities", get(handlers::capabilities))
        .route("/v1/analyze", post(analyze::analyze))
        .route("/v1/limits/author", post(authoring::author))
        .route("/v1/solve", post(authoring::solve))
        .route("/v1/curve", post(pipeline::curve))
        .route("/v1/surface", post(graphs::surface))
        .route("/v1/transcribe", post(pipeline::transcribe))
        .route("/v1/pipeline", get(pipeline::info))
        .with_state(state)
}
