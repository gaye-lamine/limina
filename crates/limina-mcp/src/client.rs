//! The HTTP client of the Limina API.
//!
//! It sends a request and hands back the status and the JSON body exactly as
//! received. It carries no credential: the only setting is the base URL.

use std::time::Duration;

use serde_json::Value;

/// Time allowed to one API call. `POST /v1/analyze` has its own 60 s deadline
/// on the server, so this is a little longer.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);

/// What the API answered.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiReply {
    /// HTTP status code.
    pub status: u16,
    /// The JSON body, as received.
    pub body: Value,
}

/// Why a call to the API gave no reply.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The HTTP client could not be built.
    #[error("cannot build the HTTP client: {0}")]
    Build(String),
    /// The server did not answer.
    #[error("cannot reach the Limina API at {url}: {reason}")]
    Unreachable {
        /// The URL that was called.
        url: String,
        /// The transport error.
        reason: String,
    },
    /// The server answered with a body that is not JSON.
    #[error("the Limina API answered HTTP {status} with a body that is not JSON")]
    NotJson {
        /// HTTP status code.
        status: u16,
    },
}

/// A client of the Limina HTTP API.
#[derive(Debug, Clone)]
pub struct ApiClient {
    base: String,
    http: reqwest::Client,
}

impl ApiClient {
    /// Builds a client for `base`, for example `http://127.0.0.1:3000`.
    ///
    /// Errors: `Build` if the HTTP client cannot be created.
    pub fn new(base: &str) -> Result<ApiClient, ClientError> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| ClientError::Build(e.to_string()))?;
        Ok(ApiClient {
            base: base.trim_end_matches('/').to_string(),
            http,
        })
    }

    /// The base URL, without a trailing slash.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// `GET <base><path>`.
    pub async fn get(&self, path: &str) -> Result<ApiReply, ClientError> {
        let url = format!("{}{}", self.base, path);
        let sent = self.http.get(&url).send().await;
        Self::read(url, sent).await
    }

    /// `POST <base><path>` with `body` as JSON.
    pub async fn post(&self, path: &str, body: &Value) -> Result<ApiReply, ClientError> {
        let url = format!("{}{}", self.base, path);
        let sent = self
            .http
            .post(&url)
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await;
        Self::read(url, sent).await
    }

    async fn read(
        url: String,
        sent: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<ApiReply, ClientError> {
        let unreachable = |e: reqwest::Error| ClientError::Unreachable {
            url: url.clone(),
            reason: e.to_string(),
        };
        let response = sent.map_err(unreachable)?;
        let status = response.status().as_u16();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ClientError::Unreachable {
                url: url.clone(),
                reason: e.to_string(),
            })?;
        let body = serde_json::from_slice(&bytes).map_err(|_| ClientError::NotJson { status })?;
        Ok(ApiReply { status, body })
    }
}
