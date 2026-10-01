#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Entity definitions and entity data — read-only.
//!
//! Responses stay `serde_json::Value` on purpose: entity shapes are defined by
//! users at runtime, so there is no static type to deserialize into.

use serde_json::json;

use super::{ClientError, Envelope, RdcClient};
use crate::auth::CallerContext;

/// Hard cap on sampled rows.
///
/// These tools exist so a model can see the shape of real data before writing
/// a mapping — not to export a table into a context window.
pub const MAX_ENTITY_SAMPLE: u32 = 100;
/// Rows returned when the caller does not ask for a specific number.
pub const DEFAULT_ENTITY_SAMPLE: u32 = 10;

impl RdcClient {
    /// # Errors
    /// Returns `ClientError` on transport failure or a non-2xx response.
    pub async fn list_entity_definitions(
        &self,
        ctx: &CallerContext,
    ) -> Result<serde_json::Value, ClientError> {
        self.get_json::<Envelope<serde_json::Value>>("/admin/api/v1/entity-definitions", ctx)
            .await?
            .require_data()
    }

    /// The field definitions for one entity type.
    ///
    /// This is what makes an entity mapping writable: without it a model
    /// guesses field names, and every entity-target workflow fails.
    ///
    /// # Errors
    /// As [`Self::list_entity_definitions`].
    pub async fn get_entity_fields(
        &self,
        entity_type: &str,
        ctx: &CallerContext,
    ) -> Result<serde_json::Value, ClientError> {
        self.get_json::<Envelope<serde_json::Value>>(
            &format!("/admin/api/v1/entity-definitions/{entity_type}/fields"),
            ctx,
        )
        .await?
        .require_data()
    }

    /// Sample rows of an entity type.
    ///
    /// `limit` is clamped rather than rejected: a model asking for 5000 rows
    /// wants to see the data, and answering with an error instead of 100 rows
    /// helps nobody.
    ///
    /// # Errors
    /// As [`Self::list_entity_definitions`].
    pub async fn query_entities(
        &self,
        entity_type: &str,
        filter: Option<&serde_json::Value>,
        limit: Option<u32>,
        ctx: &CallerContext,
    ) -> Result<serde_json::Value, ClientError> {
        let limit = limit
            .unwrap_or(DEFAULT_ENTITY_SAMPLE)
            .min(MAX_ENTITY_SAMPLE);
        let mut body = json!({ "limit": limit });
        if let (Some(filter), Some(obj)) = (filter, body.as_object_mut()) {
            obj.insert("filters".to_string(), filter.clone());
        }
        self.post_json::<_, Envelope<serde_json::Value>>(
            &format!("/api/v1/{entity_type}/query"),
            &body,
            ctx,
        )
        .await?
        .require_data()
    }
}

#[cfg(test)]
mod tests;
