#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! One request in, one response out.

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt as _, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::StreamableHttpService;

use super::guards::{bearer_of, host_allowed, origin_allowed};
use super::{ServerState, HEALTH_PATH, MCP_PATH};
use crate::auth::metadata;
use crate::tools::RdcTools;

/// The body type `rmcp`'s tower service produces.
pub(super) type ResponseBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

/// Route one request.
pub(super) async fn route(
    state: Arc<ServerState>,
    req: Request<Incoming>,
) -> Response<ResponseBody> {
    let path = req.uri().path().to_string();
    let config = state.config();

    // Before anything else, including the unauthenticated endpoints: a
    // DNS-rebinding attack aims a browser at a server it should not be able to
    // reach, and discovery metadata is exactly what it would want to read.
    if !origin_allowed(&config.allowed_origins, &req) {
        return text_response(StatusCode::FORBIDDEN, "origin not allowed");
    }
    if !host_allowed(&config.allowed_hosts, &req) {
        return text_response(StatusCode::FORBIDDEN, "host not allowed");
    }

    if path == metadata::METADATA_PATH || path == metadata::metadata_path(config) {
        return json_response(
            StatusCode::OK,
            &metadata::protected_resource_document(config),
        );
    }
    if path == HEALTH_PATH {
        return text_response(StatusCode::OK, "ok");
    }
    if path != MCP_PATH {
        return text_response(StatusCode::NOT_FOUND, "not found");
    }

    let Some(token) = bearer_of(&req) else {
        return challenge(&state);
    };

    let ctx = match state.authenticate(&token).await {
        Ok(ctx) => ctx,
        Err(reason) => {
            // Logged in full, answered with almost nothing: which part of a
            // forgery was wrong is not something to hand back.
            log::warn!("rejected an MCP caller: {reason}");
            return challenge(&state);
        }
    };

    let permissions = state.permissions_for(&ctx).await;
    let client = Arc::clone(state.client());

    // A fresh tool instance per request, holding this caller's context and
    // permissions, so no tool body knows the transport changed.
    let service = StreamableHttpService::new(
        move || {
            Ok(RdcTools::new(
                Arc::clone(&client),
                permissions.clone(),
                ctx.clone(),
            ))
        },
        Arc::new(NeverSessionManager::default()),
        state.http_config().clone(),
    );

    service.handle(req).await
}

/// A `401` carrying the discovery challenge.
///
/// The challenge is the whole point: without it a client knows only that it
/// needs a token, not where to obtain one.
pub(super) fn challenge(state: &ServerState) -> Response<ResponseBody> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            "WWW-Authenticate",
            metadata::challenge_header(state.config()),
        )
        .header("Content-Type", "text/plain")
        .body(boxed("authentication required"))
        .unwrap_or_else(|_| Response::new(boxed("authentication required")))
}

pub(super) fn json_response(
    status: StatusCode,
    body: &serde_json::Value,
) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(boxed(&body.to_string()))
        .unwrap_or_else(|_| Response::new(boxed("{}")))
}

pub(super) fn text_response(status: StatusCode, body: &str) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(boxed(body))
        .unwrap_or_else(|_| Response::new(boxed(body)))
}

fn boxed(body: &str) -> ResponseBody {
    Full::new(Bytes::from(body.to_string()))
        .map_err(|never| match never {})
        .boxed()
}
