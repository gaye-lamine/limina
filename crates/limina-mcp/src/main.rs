//! `limina-mcp`: the Limina MCP server on stdio.
//!
//! The only setting is `LIMINA_API_URL` (default `http://127.0.0.1:3000`).
//! Nothing is written to stdout except the protocol; messages go to stderr.

use std::process::ExitCode;

use limina_mcp::{ApiClient, LiminaMcp};
use rmcp::transport::stdio;
use rmcp::ServiceExt;

const DEFAULT_API_URL: &str = "http://127.0.0.1:3000";

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("limina-mcp: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("LIMINA_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    let api = ApiClient::new(&base)?;
    eprintln!("limina-mcp: calling the Limina API at {}", api.base());
    let service = LiminaMcp::new(api).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
