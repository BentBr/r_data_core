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

pub mod discover;

use std::sync::Arc;

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
}

impl RdcTools {
    #[must_use]
    pub const fn new(
        client: Arc<RdcClient>,
        permissions: Permissions,
        caller: CallerContext,
    ) -> Self {
        Self {
            client,
            permissions,
            caller,
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
