#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! What this server is, and what it is pointed at.
//!
//! A model that cannot tell which instance it is talking to writes plausible
//! nonsense into the wrong one. This is the tool it calls first when that
//! matters — and the one whose output belongs in a bug report.
//!
//! Ungated, unlike every other tool here. Everything it discloses is either
//! already known to an authenticated caller or already readable from a public
//! endpoint; the one part that is not — the deployed component versions — is
//! gated inline on `system:read` rather than by hiding the whole tool, so a
//! caller without it still learns which instance they are connected to.

use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use serde_json::json;

use super::discover::json_result;
use super::{visible_tools, RdcTools};

#[tool_router(router = system_router, vis = "pub")]
impl RdcTools {
    #[tool(
        name = "system_info",
        description = "Describe the RDataCore instance this server is connected to: its \
                       address, versions, enabled features, and what this caller may do. \
                       Call it first when which instance you are changing matters, and \
                       quote it when reporting a problem."
    )]
    pub async fn system_info(&self) -> Result<CallToolResult, ErrorData> {
        let ctx = self.caller();

        // Public endpoint, so no permission check. A failure is reported as a
        // null rather than swallowed silently: it means the instance is
        // unreachable, and every other tool is about to fail the same way.
        let capabilities = self
            .client
            .get_json::<serde_json::Value>("/admin/api/v1/system/capabilities", &ctx)
            .await
            .ok();

        // Checked rather than attempted-and-discarded: a 403 in the log for a
        // call we already knew would be refused is noise in someone's audit
        // trail.
        let versions = if self.permissions.allows("system:read") {
            self.client
                .get_json::<serde_json::Value>("/admin/api/v1/system/versions", &ctx)
                .await
                .ok()
        } else {
            None
        };

        Ok(json_result(&json!({
            "mcp_server": {
                "name": "r-data-core-mcp",
                "version": env!("CARGO_PKG_VERSION"),
                "protocol_version": rmcp::model::ProtocolVersion::LATEST,
            },
            "instance": {
                "base_url": self.client.base_url(),
                "capabilities": capabilities,
                "versions": versions,
            },
            "caller": {
                "is_super_admin": self.permissions.is_super_admin,
                "permissions": self.permissions.entries,
                "available_tools": visible_tools(&self.permissions),
            },
        })))
    }
}

#[cfg(test)]
mod tests;
