#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, unsafe_code)]

//! The streamable-HTTP MCP server.
//!
//! Multi-user by nature: every request carries its own caller's token, and
//! the server holds no credential of its own. That is why configuration
//! refuses a static `RDC_API_KEY` on this transport — one shared key would
//! authenticate every caller as the same account and collapse the per-user
//! permissions the whole design rests on.

use std::sync::Arc;

use r_data_core_mcp::config::{Config, Transport};
use r_data_core_mcp::http::{self, ServerState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = Config::from_env()?;

    let Transport::Http { ref bind } = config.transport else {
        return Err("this binary serves HTTP only; use r-data-core-mcp for stdio mode".into());
    };
    let bind = bind.clone();

    if config.allowed_origins.is_empty() {
        // Not fatal — a non-browser deployment has no Origin to check — but an
        // operator who expected the protection should know it is not running.
        log::warn!(
            "no allowed origins configured, so Origin validation is off. Set \
             RDC_MCP_ALLOWED_ORIGINS if browsers reach this server."
        );
    }

    let state = Arc::new(ServerState::new(config.clone())?);
    let listener = http::bind(&bind).await?;
    http::log_startup(&config, listener.local_addr()?);

    let shutdown = tokio_util::sync::CancellationToken::new();
    let signal = shutdown.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            log::info!("shutting down");
            signal.cancel();
        }
    });

    http::serve(state, listener, shutdown).await
}
