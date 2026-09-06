#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The tool surface, and who is offered which parts of it.
//!
//! Two things are worth knowing before reading further.
//!
//! **There is no delete tool.** Not a guarded one, not a disabled one — the
//! capability does not exist in this binary, so no amount of prompting reaches
//! it.
//!
//! **Permission filtering here is ergonomics, not security.** It stops a
//! read-only caller being offered `create_workflow` and then hitting a 403 they
//! could not have avoided. `RDataCore` performs the real check on every call,
//! and a tool invoked despite being hidden still fails there.

pub mod author;
pub mod discover;

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};

use crate::auth::{CallerContext, Permissions};
use crate::client::RdcClient;

/// The MCP tool surface.
///
/// Tool methods are attached in `discover`, `author`, `execute` and `schedule`.
pub struct RdcTools {
    pub(crate) client: Arc<RdcClient>,
    pub(crate) permissions: Permissions,
    /// The caller this instance acts for.
    ///
    /// stdio has exactly one caller, fixed at construction. The HTTP transport
    /// in the follow-up plan replaces this per request. Tool bodies must always
    /// reach it through [`RdcTools::caller`] and never build a context
    /// themselves, so that swap touches nothing else.
    pub(crate) caller: CallerContext,
    /// The tools this caller may use.
    ///
    /// Built from every tool, then narrowed with `disable_route` for anything
    /// this caller lacks permission for. `disable_route` both hides a tool
    /// from listings *and* rejects calls to it, so the narrowing closes the
    /// invoke path rather than merely tidying the menu.
    pub(crate) tool_router: ToolRouter<Self>,
}

impl RdcTools {
    #[must_use]
    pub fn new(client: Arc<RdcClient>, permissions: Permissions, caller: CallerContext) -> Self {
        let mut tool_router = Self::discover_router() + Self::author_router();
        for tool in ALL_TOOLS {
            if !is_tool_visible(tool, &permissions) {
                tool_router.disable_route(*tool);
            }
        }
        Self {
            client,
            permissions,
            caller,
            tool_router,
        }
    }

    /// The caller on whose behalf the current tool call is running.
    pub(crate) fn caller(&self) -> CallerContext {
        self.caller.clone()
    }

    /// The tools this instance's caller may use.
    ///
    /// The `ServerHandler` implementation filters its advertised list through
    /// this, so a caller is never offered something they will only be refused.
    #[must_use]
    pub fn visible_tool_names(&self) -> Vec<&'static str> {
        visible_tools(&self.permissions)
    }
}

/// Every tool this server exposes.
///
/// Listing them in one place keeps the permission map and the tool set from
/// drifting apart — `every_tool_has_a_permission_decision` asserts it.
pub const ALL_TOOLS: &[&str] = &[
    // discover
    "list_workflows",
    "get_workflow",
    "list_entity_definitions",
    "get_entity_definition",
    "query_entities",
    "dsl_options",
    // author
    "validate_dsl",
    "create_workflow",
    "update_workflow",
    // versions
    "list_workflow_versions",
    "get_workflow_version",
    "restore_workflow_version",
    // execute and debug
    "test_workflow",
    "run_workflow",
    "list_runs",
    "get_run_logs",
    // schedule
    "preview_cron",
];

/// The permission a tool needs, if any.
///
/// Strings are `{namespace}:{action}` exactly as `AuthUserClaims.permissions`
/// spells them — namespaces from `ResourceNamespace::as_str`, actions from the
/// lowercase forms in `generate_workflow_permissions`. Getting a name wrong
/// here silently hides a tool, so they are pinned by test.
#[must_use]
pub fn required_permission(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "list_workflows"
        | "get_workflow"
        | "list_workflow_versions"
        | "get_workflow_version"
        | "list_runs"
        | "get_run_logs"
        | "validate_dsl"
        | "dsl_options"
        | "preview_cron" => Some("workflows:read"),

        "create_workflow" => Some("workflows:create"),

        // The dry-run endpoint is gated on Update, because it executes.
        "update_workflow" | "restore_workflow_version" | "test_workflow" => {
            Some("workflows:update")
        }

        "run_workflow" => Some("workflows:execute"),

        "list_entity_definitions" | "get_entity_definition" => Some("entity_definitions:read"),
        "query_entities" => Some("entities:read"),

        _ => None,
    }
}

/// Whether a tool should be advertised to this caller.
#[must_use]
pub fn is_tool_visible(tool_name: &str, permissions: &Permissions) -> bool {
    required_permission(tool_name).is_none_or(|needed| permissions.allows(needed))
}

/// The tools this caller can use, in declaration order.
#[must_use]
pub fn visible_tools(permissions: &Permissions) -> Vec<&'static str> {
    ALL_TOOLS
        .iter()
        .copied()
        .filter(|name| is_tool_visible(name, permissions))
        .collect()
}

#[cfg(test)]
mod tests;

// `tool_handler` generates an async `list_tools` with no awaits in it. The
// trait requires the async signature, and the body is not ours to change.
#[allow(clippy::unused_async_trait_impl)]
#[rmcp::tool_handler(router = self.tool_router)]
impl ServerHandler for RdcTools {
    fn get_info(&self) -> ServerInfo {
        // ServerInfo and Implementation are #[non_exhaustive], so they are
        // built by mutation rather than a struct literal.
        let mut implementation = Implementation::default();
        implementation.name = "r-data-core-mcp".to_string();
        implementation.version = env!("CARGO_PKG_VERSION").to_string();

        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::LATEST;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
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
}
