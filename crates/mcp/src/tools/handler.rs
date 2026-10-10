#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The `ServerHandler` implementation: what the server says it is, and the
//! two request kinds the tool and prompt macros do not generate.
//!
//! Kept out of `tools.rs` so that file stays a map of the tool surface rather
//! than a mix of the surface and the protocol plumbing that serves it.

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    Implementation, ListResourcesResult, PaginatedRequestParams, ProtocolVersion,
    ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, ResourceContents,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer};

use super::RdcTools;
use crate::client::dsl::OptionsKind;
use crate::resources;

impl RdcTools {
    /// The body of one resource, by URI.
    ///
    /// Split out of `read_resource` because `RequestContext` cannot be built
    /// outside the service layer, and a resource body that cannot be tested
    /// is one that silently rots.
    ///
    /// # Errors
    /// `resource_not_found` for an unknown URI; `internal_error` when the
    /// live catalogue the reference is generated from cannot be read.
    pub(crate) async fn resource_text(&self, uri: &str) -> Result<String, ErrorData> {
        match uri {
            resources::DSL_RULES_URI => Ok(resources::validation_rules().to_string()),
            resources::DSL_REFERENCE_URI => self.render_reference().await,
            other => Err(ErrorData::resource_not_found(
                format!("no resource at {other}"),
                None,
            )),
        }
    }

    /// Render the DSL reference from the live catalogue.
    ///
    /// One request per group, because that is the shape the API offers. They
    /// run in sequence: three small reads, on a document a client fetches
    /// once, are not worth the concurrency.
    async fn render_reference(&self) -> Result<String, ErrorData> {
        let ctx = self.caller();
        let mut groups = Vec::with_capacity(3);
        for (label, kind) in [
            ("from", OptionsKind::From),
            ("transform", OptionsKind::Transform),
            ("to", OptionsKind::To),
        ] {
            let options = self
                .client
                .dsl_options(kind, &ctx)
                .await
                .map_err(|e| ErrorData::internal_error(e.to_tool_message(), None))?;
            groups.push((label, options));
        }
        Ok(resources::render_dsl_reference(&groups))
    }
}

// `tool_handler` generates an async `list_tools` with no awaits in it. The
// trait requires the async signature, and the body is not ours to change.
#[allow(clippy::unused_async_trait_impl)]
#[rmcp::tool_handler(router = self.tool_router)]
#[rmcp::prompt_handler(router = self.prompt_router)]
impl ServerHandler for RdcTools {
    fn get_info(&self) -> ServerInfo {
        // ServerInfo and Implementation are #[non_exhaustive], so they are
        // built by mutation rather than a struct literal.
        let mut implementation = Implementation::default();
        implementation.name = "r-data-core-mcp".to_string();
        implementation.version = env!("CARGO_PKG_VERSION").to_string();

        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::LATEST;
        info.capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_prompts()
            .enable_resources()
            .build();
        info.server_info = implementation;
        info.instructions = Some(
            "Author, run and debug RDataCore workflows.\n\n\
                 Before writing a DSL program, call dsl_options for the parts you \
                 need — it is generated from the running engine and is the only \
                 accurate description of the language. Do not rely on remembered \
                 DSL syntax.\n\n\
                 For a workflow that touches entities, call get_entity_definition \
                 first; do not guess field names.\n\n\
                 Always validate_dsl and then test_workflow before saving. \
                 test_workflow executes for real against an in-memory overlay and \
                 persists nothing, so iterate there freely. run_workflow, by \
                 contrast, has real side effects."
                .to_string(),
        );
        info
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(resources::descriptors()))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let text = self.resource_text(&request.uri).await?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(text, request.uri)]).into())
    }
}
