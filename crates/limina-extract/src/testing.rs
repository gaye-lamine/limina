//! Test support, public so the API crate can test without a network: a
//! scripted mock provider and the catalog of the served limits.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::catalog::{Catalog, CatalogAssumption, CatalogLimit, CatalogOption, CatalogParameter};
use crate::provider::{BoxFuture, ChatReply, ChatRequest, ExtractionProvider, ProviderError};

/// A provider that answers from a queue and records what it was sent.
/// With an empty queue it answers HTTP 500.
#[derive(Debug, Default)]
pub struct MockProvider {
    replies: Mutex<VecDeque<Result<ChatReply, ProviderError>>>,
    requests: Mutex<Vec<ChatRequest>>,
}

impl MockProvider {
    /// An empty mock.
    pub fn new() -> MockProvider {
        MockProvider::default()
    }

    /// Queues one answer.
    pub fn push(&self, reply: Result<ChatReply, ProviderError>) {
        self.replies
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push_back(reply);
    }

    /// Queues one successful answer with this content.
    pub fn push_content(&self, content: &str) {
        self.push(Ok(ChatReply {
            content: Some(content.to_string()),
            finish_reason: Some("stop".to_string()),
        }));
    }

    /// How many requests were sent.
    pub fn calls(&self) -> usize {
        self.requests
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }

    /// Every request sent, in order.
    pub fn requests(&self) -> Vec<ChatRequest> {
        self.requests
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

impl ExtractionProvider for MockProvider {
    fn model(&self) -> &str {
        "mock-model"
    }

    fn complete(&self, request: ChatRequest) -> BoxFuture<'_, Result<ChatReply, ProviderError>> {
        self.requests
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(request);
        let reply = self
            .replies
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .pop_front()
            .unwrap_or(Err(ProviderError::Status(500)));
        Box::pin(async move { reply })
    }
}

fn option(id: &str, label: &str) -> CatalogOption {
    CatalogOption {
        id: id.into(),
        label: label.into(),
    }
}

fn temperatures() -> Vec<CatalogParameter> {
    ["Th", "Tc"]
        .iter()
        .map(|n| CatalogParameter {
            name: (*n).into(),
            units: vec!["K".into()],
        })
        .collect()
}

/// The catalog of betz, carnot and cop, as the API builds it.
pub fn catalog() -> Catalog {
    Catalog {
        limits: vec![
            CatalogLimit {
                id: "betz".into(),
                title: "Betz limit".into(),
                formula: "Cp <= 16/27".into(),
                claim_unit: "dimensionless".into(),
                parameters: vec![],
                assumptions: vec![CatalogAssumption {
                    id: "reference_area".into(),
                    options: vec![
                        option("unshrouded_rotor", "unshrouded rotor"),
                        option(
                            "shrouded_total_capture_area",
                            "shrouded, total capture area",
                        ),
                        option("shrouded_rotor_area_only", "shrouded, rotor area only"),
                    ],
                }],
            },
            CatalogLimit {
                id: "carnot".into(),
                title: "Carnot limit".into(),
                formula: "efficiency <= 1 - Tc/Th".into(),
                claim_unit: "dimensionless".into(),
                parameters: temperatures(),
                assumptions: vec![CatalogAssumption {
                    id: "cycle_model".into(),
                    options: vec![option(
                        "ideal_reversible_two_reservoirs",
                        "ideal reversible cycle between two reservoirs",
                    )],
                }],
            },
            CatalogLimit {
                id: "cop".into(),
                title: "Carnot COP limit".into(),
                formula: "COP <= Th/(Th-Tc) (heating), COP <= Tc/(Th-Tc) (cooling)".into(),
                claim_unit: "dimensionless".into(),
                parameters: temperatures(),
                assumptions: vec![CatalogAssumption {
                    id: "mode".into(),
                    options: vec![option("heating", "heating"), option("cooling", "cooling")],
                }],
            },
        ],
    }
}
