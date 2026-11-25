//! The provider trait: the only way this crate reaches a language model.

use std::future::Future;
use std::pin::Pin;

/// A boxed future, so the trait can be used as `dyn ExtractionProvider`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One chat completion: a system prompt and one user message.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    /// System prompt, built from the limits metadata.
    pub system: String,
    /// User message: the text inside its delimited data block.
    pub user: String,
}

/// What the provider returned. Only `message.content` is read, never any
/// reasoning field.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatReply {
    /// `choices[0].message.content`.
    pub content: Option<String>,
    /// `choices[0].finish_reason`.
    pub finish_reason: Option<String>,
}

/// A provider failure. It never carries the response body, the headers or
/// the key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    /// The endpoint answered with a non-success HTTP status.
    #[error("the extraction provider answered with HTTP status {0}")]
    Status(u16),
    /// No answer within the timeout.
    #[error("the extraction provider did not answer in time")]
    Timeout,
    /// The request could not be sent or the answer could not be read.
    #[error("the extraction provider could not be reached")]
    Transport,
}

impl ProviderError {
    /// HTTP 429 and 5xx are retried once.
    pub fn is_retryable(&self) -> bool {
        matches!(self, ProviderError::Status(s) if *s == 429 || (500..=599).contains(s))
    }
}

/// A language model that completes one chat.
pub trait ExtractionProvider: Send + Sync {
    /// Model id, reported with every extraction.
    fn model(&self) -> &str;

    /// Sends one chat completion request.
    fn complete(&self, request: ChatRequest) -> BoxFuture<'_, Result<ChatReply, ProviderError>>;
}

/// An image sent to a vision-language model: its MIME type and its bytes
/// in base64.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInput {
    /// `image/png`, `image/jpeg` or `image/webp`.
    pub mime: String,
    /// The image bytes, base64 encoded.
    pub base64: String,
}

/// A vision-language model that transcribes the text of an image. The
/// transcription is then analysed like any pasted text; the image itself
/// never reaches the engine.
pub trait VisionProvider: Send + Sync {
    /// Model id.
    fn model(&self) -> &str;
    /// The text written in the image, exactly as written.
    fn transcribe(&self, image: ImageInput) -> BoxFuture<'_, Result<ChatReply, ProviderError>>;
}
