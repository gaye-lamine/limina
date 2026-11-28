//! The Limina MCP server.
//!
//! It exposes five tools over the Model Context Protocol (stdio) and does
//! nothing else than call the Limina HTTP API: every verdict, bound and
//! threshold in a tool result is the API's answer, passed on unchanged. It
//! holds no key and reads no setting other than the API's base URL.

pub mod client;
pub mod server;

pub use client::{ApiClient, ApiReply, ClientError};
pub use server::LiminaMcp;
