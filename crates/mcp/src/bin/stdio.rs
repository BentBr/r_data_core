#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, unsafe_code)]

//! The stdio MCP server.
//!
//! Single-user by nature: everything runs as the owner of the configured
//! credential. That is fine for a developer on their own machine, and is why
//! configuration refuses this mode's credential over the HTTP transport, where
//! callers differ.

use std::sync::Arc;

use rmcp::transport::stdio;
use rmcp::ServiceExt;

use r_data_core_mcp::auth::{ApiKeyBackend, CallerContext, Permissions};
use r_data_core_mcp::client::RdcClient;
use r_data_core_mcp::config::{Config, Transport};
use r_data_core_mcp::tools::RdcTools;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logging goes to stderr: stdout carries the JSON-RPC stream, and a stray
    // log line there corrupts the protocol.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    let config = Config::from_env()?;

    let Transport::Stdio = config.transport else {
        return Err(
            "this binary serves stdio only; use r-data-core-mcp-serve for HTTP mode".into(),
        );
    };

    let credential = config
        .credential
        .clone()
        .ok_or("RDC_API_KEY is required for stdio transport")?;

    let client = Arc::new(RdcClient::new(
        &config,
        Arc::new(ApiKeyBackend::new(credential)),
    )?);

    // Resolve permissions once, so the caller is offered only tools they can
    // use. Failure is fatal, deliberately: neither fallback is good. Assuming
    // full access invites a stream of 403s; assuming none advertises almost
    // nothing and looks like a broken server. If permissions cannot be read
    // the credential is almost certainly wrong, and saying so beats either.
    let permissions = match client.permissions(&CallerContext::default()).await {
        Ok(body) => Permissions::from_api_response(&body),
        Err(e) => {
            return Err(format!(
                "could not load permissions from {}: {}. Check RDC_API_KEY and that the \
                 key has not been revoked.",
                config.base_url,
                e.to_tool_message()
            )
            .into())
        }
    };

    let tools = RdcTools::new(client, permissions, CallerContext::default());
    log::info!(
        "serving {} tool(s) over stdio against {}",
        tools.visible_tool_names().len(),
        config.base_url
    );

    let service = tools.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
