#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! DSL catalogue, validation and dry-run endpoints.

use serde_json::json;

use r_data_core_core::dto::dsl::{
    DryRunResponse, DslOptionsAndExamplesResponse, DslValidateResponse,
};

use super::{ClientError, Envelope, RdcClient};
use crate::auth::CallerContext;

/// Which part of the DSL vocabulary to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsKind {
    From,
    To,
    Transform,
}

impl OptionsKind {
    /// Parse a caller-supplied kind, rejecting anything else by name.
    ///
    /// # Errors
    /// Returns a `Validation` error listing the three legal values — the
    /// teaching-error principle applied to the tool's own input, so a model
    /// that guesses "source" is told what to say instead.
    pub fn parse(raw: &str) -> Result<Self, ClientError> {
        match raw {
            "from" => Ok(Self::From),
            "to" => Ok(Self::To),
            "transform" => Ok(Self::Transform),
            other => Err(ClientError::Validation {
                json_path: Some("kind".to_string()),
                message: format!("unknown DSL option kind '{other}'"),
                legal_values: vec![
                    "from".to_string(),
                    "to".to_string(),
                    "transform".to_string(),
                ],
            }),
        }
    }

    const fn as_path(self) -> &'static str {
        match self {
            Self::From => "from",
            Self::To => "to",
            Self::Transform => "transform",
        }
    }
}

impl RdcClient {
    /// The live option catalogue for one part of the DSL.
    ///
    /// This, not `docs/DSL.md`, is the authoritative vocabulary: it is built
    /// from the same structs the executor consumes, so it cannot drift.
    ///
    /// # Errors
    /// Returns `ClientError` on transport failure or a non-2xx response.
    pub async fn dsl_options(
        &self,
        kind: OptionsKind,
        ctx: &CallerContext,
    ) -> Result<DslOptionsAndExamplesResponse, ClientError> {
        self.get_json::<Envelope<DslOptionsAndExamplesResponse>>(
            &format!("/admin/api/v1/dsl/{}/options", kind.as_path()),
            ctx,
        )
        .await?
        .require_data()
    }

    /// Validate a program without saving it.
    ///
    /// A valid program answers 200; an invalid one answers 422, which the
    /// client layer has already turned into a located `ClientError::Validation`.
    /// There is therefore no `valid: false` case to handle here.
    ///
    /// # Errors
    /// As [`Self::dsl_options`].
    pub async fn validate_dsl(
        &self,
        steps: &serde_json::Value,
        ctx: &CallerContext,
    ) -> Result<DslValidateResponse, ClientError> {
        self.post_json::<_, Envelope<DslValidateResponse>>(
            "/admin/api/v1/dsl/validate",
            &json!({ "steps": steps }),
            ctx,
        )
        .await?
        .require_data()
    }

    /// Execute a program against sample input, changing nothing.
    ///
    /// # Errors
    /// As [`Self::dsl_options`].
    pub async fn dry_run(
        &self,
        steps: &serde_json::Value,
        input: &serde_json::Value,
        ctx: &CallerContext,
    ) -> Result<DryRunResponse, ClientError> {
        self.post_json::<_, Envelope<DryRunResponse>>(
            "/admin/api/v1/dsl/dry-run",
            &json!({ "steps": steps, "input": input }),
            ctx,
        )
        .await?
        .require_data()
    }
}

#[cfg(test)]
mod tests;
