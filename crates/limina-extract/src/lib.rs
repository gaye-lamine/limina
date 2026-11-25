//! Reads physical performance claims out of a text through a language model,
//! then checks every value against the text without any model.
//!
//! The language model only reads numbers; it never decides a verdict. Every
//! evidence must be an exact quotation of the text and every number must
//! appear in its evidence, otherwise the value is dropped and flagged. The
//! verdict is computed by `limina-core`, called by `limina-api`.
//!
//! Language models are reached only through [`ExtractionProvider`]; the one
//! implementation, [`OpenAiProvider`], is the only provider-specific code.

pub mod author;
pub mod catalog;
pub mod check;
pub mod config;
pub mod error;
pub mod extractor;
pub mod law;
pub mod openai;
pub mod parse;
pub mod prompt;
pub mod provider;
pub mod testing;

pub use author::{author_limit, author_system_prompt, AuthorError, AuthoredLimit};
pub use catalog::{Catalog, CatalogAssumption, CatalogLimit, CatalogOption, CatalogParameter};
pub use check::{
    CheckedClaim, CheckedParameter, Span, CONSTRAINT_NOT_STATED, LIMIT_NOT_RECOGNIZED,
    NOT_TRACEABLE, PARAMETER_NOT_DECLARED, UNIT_NOT_UNDERSTOOD, VALUE_NOT_TRACEABLE,
};
pub use config::{ExtractConfig, SecretKey};
pub use error::ExtractError;
pub use extractor::{Extraction, Extractor};
pub use law::{
    author_two, extract_parameters, identify_law, transcribe_image, FoundParameter, LawStatus,
    ModelAttempt, TwoModelLaw, WantedParameter, NO_LAW,
};
pub use openai::OpenAiProvider;
pub use parse::ModelOutput;
pub use provider::{
    BoxFuture, ChatReply, ChatRequest, ExtractionProvider, ImageInput, ProviderError,
    VisionProvider,
};
