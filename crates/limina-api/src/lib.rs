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
pub mod handlers;
pub mod labels;
pub mod registry;
pub mod response;

use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use limina_dsl::DeclarativeLimit;

use axum::routing::{get, post};
use axum::Router;
use limina_extract::{ExtractConfig, ExtractError, ExtractionProvider, Extractor, OpenAiProvider};

/// Shared state: the extractor, present only when a key is configured, and
/// the limits authored while the server runs (in memory, never persisted).
#[derive(Clone, Default)]
pub struct AppState {
    extractor: Option<Arc<Extractor>>,
    authored: Arc<RwLock<Vec<DeclarativeLimit>>>,
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
        Ok(AppState::with_provider(Arc::new(OpenAiProvider::new(
            config,
        )?)))
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
            authored: Arc::default(),
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
        .with_state(state)
}
