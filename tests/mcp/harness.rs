#![allow(clippy::expect_used, clippy::unwrap_used)]

//! A real `RDataCore` listening on a port, plus an MCP tool surface pointed at it.
//!
//! The MCP client speaks HTTP through reqwest, so it cannot use actix's
//! in-process `test::init_service` app the way the rest of the suite does. This
//! binds the same application state to an ephemeral port instead.
//!
//! That distinction is the whole reason these tests exist. The crate's own
//! tests mock the server with `wiremock` and prove the client behaves correctly
//! against the shapes it *expects*; only a real server proves those shapes are
//! the ones `RDataCore` actually produces. That gap has already bitten once — the
//! run-logs path in the `OpenAPI` annotation did not match the route.

use std::collections::HashMap;
use std::sync::Arc;

use actix_web::{App, HttpServer};

use r_data_core_api::api_state::configure_app;
use r_data_core_mcp::auth::{ApiKeyBackend, CallerContext, Permissions};
use r_data_core_mcp::client::RdcClient;
use r_data_core_mcp::config::Config;
use r_data_core_mcp::tools::RdcTools;
use r_data_core_test_support::TestDatabase;

use crate::api::workflows::common::build_app_state;

/// A running server and the pieces needed to talk to it.
pub struct McpTestStack {
    pub base_url: String,
    pub pool: TestDatabase,
    pub api_key: String,
    /// Kept so the server task is aborted when the stack is dropped; a leaked
    /// listener would hold its port for the rest of the run.
    _server: ServerGuard,
}

struct ServerGuard(tokio::task::JoinHandle<()>);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl McpTestStack {
    /// Start a real server on an ephemeral port.
    ///
    /// # Panics
    /// Panics if the application state cannot be built or the port cannot be
    /// bound — either means the test environment is broken, and failing loudly
    /// beats a confusing connection error later.
    pub async fn start() -> Self {
        let (app_data, pool, _token, api_key) =
            build_app_state().await.expect("build application state");

        // Port 0: let the OS pick, so parallel tests cannot collide.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
        let port = listener.local_addr().expect("read the bound port").port();

        let server = HttpServer::new(move || {
            App::new()
                .app_data(app_data.clone())
                .configure(configure_app)
        })
        .listen(listener)
        .expect("attach the server to the listener")
        .workers(1)
        .run();

        let handle = tokio::spawn(async move {
            let _ = server.await;
        });

        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            pool,
            api_key,
            _server: ServerGuard(handle),
        }
    }

    /// A client authenticated with the stack's API key.
    pub fn client(&self) -> Arc<RdcClient> {
        let mut map = HashMap::new();
        map.insert("RDC_BASE_URL".to_string(), self.base_url.clone());
        map.insert("RDC_API_KEY".to_string(), self.api_key.clone());
        let config = Config::from_map(&map).expect("config");
        Arc::new(
            RdcClient::new(
                &config,
                Arc::new(ApiKeyBackend::new(&config, self.api_key.clone()).expect("auth backend")),
            )
            .expect("client"),
        )
    }

    /// The tool surface, with permissions resolved from the live server.
    ///
    /// Resolving rather than assuming is deliberate: it exercises the same
    /// path the binary uses at startup, so a mismatch between what the server
    /// reports and what the tool registry expects surfaces here.
    pub async fn tools(&self) -> RdcTools {
        let client = self.client();
        let permissions = client
            .permissions(&CallerContext::default())
            .await
            .map(|body| Permissions::from_api_response(&body))
            .expect("the live server should report permissions");
        RdcTools::new(client, permissions, CallerContext::default())
    }
}

/// The text a tool result carries, whether success or failure.
pub fn text_of(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|block| block.as_text().map(|t| t.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}
