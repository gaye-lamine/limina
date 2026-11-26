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
pub mod qualified;
pub mod registry;
pub mod response;

use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use limina_dsl::DeclarativeLimit;

use axum::routing::{get, post};
use axum::Router;
use limina_extract::{
    extraction_problem, ExtractConfig, ExtractError, ExtractionProvider, Extractor,
    FallbackProvider, FallbackVision, OpenAiProvider, ReplyKind, VisionProvider, VISION_TIMEOUT,
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
    chains: Option<dto::ChainsDto>,
    analyze_timeout: Option<Duration>,
}

/// Default deadline of one `POST /v1/analyze` request
/// (`LIMINA_ANALYZE_TIMEOUT_SECS` overrides it).
pub const DEFAULT_ANALYZE_TIMEOUT: Duration = Duration::from_secs(60);

/// One OpenAI-compatible provider per model id of `models`.
fn links(
    config: &ExtractConfig,
    models: &[String],
) -> Result<Vec<Arc<OpenAiProvider>>, ExtractError> {
    models
        .iter()
        .map(|m| OpenAiProvider::new(&config.with_model(m)).map(Arc::new))
        .collect()
}

fn chat_chain(
    config: &ExtractConfig,
    models: &[String],
    kind: ReplyKind,
) -> Result<FallbackProvider, ExtractError> {
    let links: Vec<Arc<dyn ExtractionProvider>> = links(config, models)?
        .into_iter()
        .map(|l| l as Arc<dyn ExtractionProvider>)
        .collect();
    Ok(FallbackProvider::new(links, kind, config.timeout()))
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
        let json = chat_chain(
            config,
            config.models(),
            ReplyKind::Checked(extraction_problem),
        )?;
        let parameters = chat_chain(config, config.models(), ReplyKind::Json)?;
        let text = chat_chain(config, config.models(), ReplyKind::Text)?;
        let second = chat_chain(config, config.models_b(), ReplyKind::Json)?;
        let vision_links: Vec<Arc<dyn VisionProvider>> = links(config, config.vision_models())?
            .into_iter()
            .map(|l| l as Arc<dyn VisionProvider>)
            .collect();
        let vision = FallbackVision::new(vision_links, VISION_TIMEOUT);
        let chains = dto::ChainsDto {
            extraction: json.models(),
            authoring_b: second.models(),
            vision: vision.models(),
        };
        let extractor = Extractor::new(Arc::new(json), registry::catalog())
            .with_text_provider(Arc::new(text))
            .with_json_provider(Arc::new(parameters));
        let mut state = AppState::with_extractor(extractor)
            .with_second_model(Arc::new(second))
            .with_vision(Arc::new(vision));
        state.chains = Some(chains);
        Ok(state)
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
            chains: self.chains.clone(),
        }
    }

    /// [`AppState::from_config`] on the process environment, with the
    /// analysis deadline of `LIMINA_ANALYZE_TIMEOUT_SECS` when it is a
    /// positive number of seconds.
    pub fn from_env() -> Result<AppState, ExtractError> {
        let state = AppState::from_config(&ExtractConfig::from_env())?;
        let secs = std::env::var("LIMINA_ANALYZE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .filter(|s| *s > 0);
        Ok(match secs {
            Some(s) => state.with_analyze_timeout(Duration::from_secs(s)),
            None => state,
        })
    }

    /// Sets the deadline of one `POST /v1/analyze` request.
    pub fn with_analyze_timeout(mut self, timeout: Duration) -> AppState {
        self.analyze_timeout = Some(timeout);
        self
    }

    /// The deadline of one `POST /v1/analyze` request.
    pub fn analyze_timeout(&self) -> Duration {
        self.analyze_timeout.unwrap_or(DEFAULT_ANALYZE_TIMEOUT)
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
