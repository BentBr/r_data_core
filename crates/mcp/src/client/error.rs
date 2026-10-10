#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Errors written to be read by a model.
//!
//! This is the highest-leverage code in the crate. An assistant told
//! `400 Bad Request` guesses; one told which key is wrong and what the legal
//! values are fixes its program in a single turn. Every message here is
//! written for that reader.
//!
//! Two rules shape the text:
//!
//! * **Say where, not just what.** The server supplies a JSON path and the
//!   legal alternatives for DSL failures; this type carries them through
//!   rather than re-deriving them from prose.
//! * **Never invite a retry that cannot succeed.** A 403 is not a transient
//!   condition, and a message that sounds like one produces a loop.

use std::fmt::Write as _;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("cannot reach RDataCore at {url}")]
    Unreachable { url: String },
    #[error("credential invalid or expired")]
    Unauthorized,
    #[error("permission denied")]
    Forbidden { permission: Option<String> },
    #[error("not found: {what}")]
    NotFound { what: String },
    #[error("validation failed: {message}")]
    Validation {
        json_path: Option<String>,
        message: String,
        legal_values: Vec<String>,
    },
    #[error("RDataCore server error: {message}")]
    Server { message: String },
    #[error("request timed out")]
    Timeout,
    #[error("could not decode the response: {message}")]
    Decode { message: String },
}

impl ClientError {
    /// Build a structured error from an HTTP status and a response body.
    ///
    /// The body is normally `RDataCore`'s `ApiResponse` envelope — `status`,
    /// `message`, `data`, `meta` — and for a validation failure a
    /// `ValidationErrorResponse`, whose `violations` sit at the **top level**
    /// beside `message`, not under `meta`. It may also be something else
    /// entirely, from an Actix error handler or a proxy, so every field access
    /// is tolerant and nothing here can panic on an unexpected shape.
    #[must_use]
    pub fn from_api_body(status: u16, body: &serde_json::Value, url: &str) -> Self {
        let message = body
            .get("message")
            .and_then(serde_json::Value::as_str)
            // Some paths, and non-RDataCore intermediaries, use `error`.
            .or_else(|| body.get("error").and_then(serde_json::Value::as_str))
            .unwrap_or("no error message")
            .to_string();

        match status {
            401 => Self::Unauthorized,
            403 => Self::Forbidden {
                permission: extract_permission(&message),
            },
            404 => Self::NotFound { what: message },
            400 | 422 => Self::from_violations(body, &message),
            500..=599 => Self::Server { message },
            _ => Self::Server {
                message: format!("unexpected status {status} from {url}: {message}"),
            },
        }
    }

    /// Prefer the server's structured violations over anything scraped from
    /// prose; fall back to the message only when there are none.
    fn from_violations(body: &serde_json::Value, message: &str) -> Self {
        let first = body
            .get("violations")
            .and_then(serde_json::Value::as_array)
            .and_then(|v| v.first());

        first.map_or_else(
            || Self::Validation {
                json_path: None,
                legal_values: legal_values_from(message),
                message: message.to_string(),
            },
            |v| {
                let detail = v
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(message)
                    .to_string();
                Self::Validation {
                    // `json_path` where the producer could manage one, else the
                    // coarser `field`.
                    json_path: v
                        .get("json_path")
                        .or_else(|| v.get("field"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    legal_values: v
                        .get("legal_values")
                        .and_then(serde_json::Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_string)
                                .collect()
                        })
                        .filter(|l: &Vec<String>| !l.is_empty())
                        // Older servers put them only in the prose.
                        .unwrap_or_else(|| legal_values_from(&detail)),
                    message: detail,
                }
            },
        )
    }

    /// Render the error for an AI assistant: what went wrong, where, and what
    /// the legal alternatives are.
    #[must_use]
    pub fn to_tool_message(&self) -> String {
        match self {
            Self::Unreachable { url } => format!(
                "Cannot reach RDataCore at {url}. Verify the URL and that the server is running."
            ),
            Self::Unauthorized => {
                "Credential invalid or expired. Create a new one in the admin panel under \
                 API Keys."
                    .to_string()
            }
            Self::Forbidden { permission } => permission.as_ref().map_or_else(
                || {
                    "Permission denied. Your account does not hold the permission this \
                     action requires."
                        .to_string()
                },
                |p| {
                    format!(
                        "Permission denied: this action needs '{p}'. Your account does not \
                         hold it. Ask an administrator to grant it."
                    )
                },
            ),
            Self::NotFound { what } => format!("Not found: {what}"),
            Self::Validation {
                json_path,
                message,
                legal_values,
            } => {
                let mut out = json_path.as_ref().map_or_else(
                    || format!("Validation failed: {message}"),
                    |p| format!("Validation failed at {p}: {message}"),
                );
                if !legal_values.is_empty() {
                    let _ = write!(out, "\nLegal values: {}", legal_values.join(", "));
                }
                out
            }
            Self::Server { message } => format!(
                "RDataCore returned a server error: {message}. This is not a problem with \
                 the request; do not retry automatically."
            ),
            Self::Timeout => "The request timed out. If this was a workflow run, it may still be \
                 executing — check its status rather than starting another run."
                .to_string(),
            Self::Decode { message } => format!("Could not decode the response: {message}"),
        }
    }
}

/// Pull the alternatives out of serde's ``expected one of `a`, `b` `` phrasing.
///
/// An empty result means "no enumerable alternatives", never "none are legal".
fn legal_values_from(message: &str) -> Vec<String> {
    let Some(tail) = message.split("expected one of ").nth(1) else {
        return Vec::new();
    };
    // Backtick-quoted names alternate with the separators between them.
    tail.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// Pull a `namespace:action` permission out of a 403 message, if present.
///
/// Messages read like "Insufficient permissions: workflows:write required", so
/// a naive search for the first token containing a colon finds the English
/// word, not the permission. A candidate must therefore have non-empty text on
/// both sides of a colon after punctuation is trimmed — which admits
/// `workflows:write` and the path-scoped `entities:/customers:read`, and
/// rejects `permissions:`.
fn extract_permission(message: &str) -> Option<String> {
    message
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| !c.is_alphanumeric() && c != ':' && c != '/'))
        .find(|token| {
            !token.contains("://")
                && token
                    .split_once(':')
                    .is_some_and(|(namespace, rest)| !namespace.is_empty() && !rest.is_empty())
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests;
