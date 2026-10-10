#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tools for finding out what exists before changing anything.
//!
//! These six are what let a model write a correct DSL program instead of
//! guessing: the live option catalogue for the vocabulary, entity definitions
//! for field names, and a sample of real rows for the shapes those fields
//! actually hold.
//!
//! Every failure here is a *tool-level* error — `CallToolResult::error` —
//! rather than `Err(ErrorData)`. MCP renders the latter opaquely, which would
//! discard exactly the text written to help a model correct itself.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::Deserialize;

use super::RdcTools;
use crate::client::dsl::OptionsKind;
use crate::client::entities::{DEFAULT_ENTITY_SAMPLE, MAX_ENTITY_SAMPLE};
use crate::client::ClientError;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListWorkflowsParams {
    /// Maximum number of workflows to return. Defaults to 50.
    pub limit: Option<u32>,
    /// Number of workflows to skip. Defaults to 0.
    pub offset: Option<u32>,
    /// Filter by enabled state. Omit to see both.
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWorkflowParams {
    /// The workflow's UUID.
    pub uuid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEntityDefinitionParams {
    /// The entity type name, e.g. "customer".
    pub entity_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryEntitiesParams {
    /// The entity type to sample, e.g. "customer".
    pub entity_type: String,
    /// Optional filter object, in the shape the query endpoint accepts.
    pub filter: Option<serde_json::Value>,
    /// Maximum rows. Defaults to 10, capped at 100.
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DslOptionsParams {
    /// Which part of the DSL to describe: "from", "to", or "transform".
    pub kind: String,
}

/// Render a successful result as pretty JSON.
///
/// Pretty rather than compact: the reader is a model, and the extra bytes buy
/// legibility in something it has to reason about structurally.
pub(super) fn json_result(value: &serde_json::Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
    )])
}

/// Render a client failure as a tool-level error the caller can read.
pub(super) fn error_result(err: &ClientError) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(err.to_tool_message())])
}

/// Parse a UUID, or explain what was expected.
pub(super) fn parse_uuid(raw: &str, field: &str) -> Result<uuid::Uuid, ClientError> {
    raw.parse().map_err(|_| ClientError::Validation {
        json_path: Some(field.to_string()),
        message: format!("'{raw}' is not a valid UUID"),
        legal_values: Vec::new(),
    })
}

#[tool_router(router = discover_router, vis = "pub")]
impl RdcTools {
    /// List the workflows in this instance.
    #[tool(
        name = "list_workflows",
        description = "List workflows with their name, kind, enabled state and schedule. \
                       Start here when asked about existing workflows."
    )]
    pub async fn list_workflows(
        &self,
        Parameters(params): Parameters<ListWorkflowsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self
            .client
            .list_workflows(params.limit, params.offset, &self.caller())
            .await;

        Ok(match result {
            Ok(workflows) => {
                // Filtering here rather than server-side: the list endpoint
                // has no enabled filter, and fetching then filtering is
                // honest about that rather than silently ignoring the flag.
                let filtered: Vec<_> = workflows
                    .into_iter()
                    .filter(|w| params.enabled.is_none_or(|want| w.enabled == want))
                    .collect();
                json_result(&serde_json::json!({ "workflows": filtered }))
            }
            Err(e) => error_result(&e),
        })
    }

    /// Fetch one workflow, including its full DSL program.
    #[tool(
        name = "get_workflow",
        description = "Get a workflow's details including its complete DSL program. Read \
                       this before proposing a change to an existing workflow."
    )]
    pub async fn get_workflow(
        &self,
        Parameters(params): Parameters<GetWorkflowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let uuid = match parse_uuid(&params.uuid, "uuid") {
            Ok(uuid) => uuid,
            Err(e) => return Ok(error_result(&e)),
        };

        Ok(match self.client.get_workflow(uuid, &self.caller()).await {
            Ok(detail) => json_result(&serde_json::to_value(detail).unwrap_or_default()),
            Err(e) => error_result(&e),
        })
    }

    /// List the entity types defined in this instance.
    #[tool(
        name = "list_entity_definitions",
        description = "List the entity types this instance defines. Use before writing a \
                       workflow that reads from or writes to entities."
    )]
    pub async fn list_entity_definitions(&self) -> Result<CallToolResult, ErrorData> {
        Ok(
            match self.client.list_entity_definitions(&self.caller()).await {
                Ok(definitions) => json_result(&definitions),
                Err(e) => error_result(&e),
            },
        )
    }

    /// The fields of one entity type.
    #[tool(
        name = "get_entity_definition",
        description = "Get an entity type's field definitions: names, types and whether \
                       each is required. A DSL mapping cannot be written correctly without \
                       these — do not guess field names."
    )]
    pub async fn get_entity_definition(
        &self,
        Parameters(params): Parameters<GetEntityDefinitionParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(
            match self
                .client
                .get_entity_fields(&params.entity_type, &self.caller())
                .await
            {
                Ok(fields) => json_result(&fields),
                Err(e) => error_result(&e),
            },
        )
    }

    /// Sample real rows of an entity type.
    #[tool(
        name = "query_entities",
        description = "Sample real rows of an entity type, so you can see the values its \
                       fields actually hold before writing a mapping. Returns at most 100 \
                       rows: this is for inspecting shape, not exporting data."
    )]
    pub async fn query_entities(
        &self,
        Parameters(params): Parameters<QueryEntitiesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = params
            .limit
            .unwrap_or(DEFAULT_ENTITY_SAMPLE)
            .min(MAX_ENTITY_SAMPLE);
        Ok(
            match self
                .client
                .query_entities(
                    &params.entity_type,
                    params.filter.as_ref(),
                    Some(limit),
                    &self.caller(),
                )
                .await
            {
                Ok(rows) => json_result(&rows),
                Err(e) => error_result(&e),
            },
        )
    }

    /// The live DSL vocabulary for one part of a step.
    #[tool(
        name = "dsl_options",
        description = "List the legal DSL types for 'from', 'to' or 'transform', with each \
                       type's fields and concrete examples. This is generated from the \
                       running engine and is the authoritative reference — prefer it over \
                       any documentation, which can be out of date."
    )]
    pub async fn dsl_options(
        &self,
        Parameters(params): Parameters<DslOptionsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let kind = match OptionsKind::parse(&params.kind) {
            Ok(kind) => kind,
            Err(e) => return Ok(error_result(&e)),
        };

        Ok(match self.client.dsl_options(kind, &self.caller()).await {
            Ok(options) => json_result(&serde_json::to_value(options).unwrap_or_default()),
            Err(e) => error_result(&e),
        })
    }
}

#[cfg(test)]
mod tests;
