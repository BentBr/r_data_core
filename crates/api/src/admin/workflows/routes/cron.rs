#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::auth::auth_enum::RequiredAuth;
use crate::response::ApiResponse;
use actix_web::{get, web, Responder};
use r_data_core_core::utils;

/// Preview next run times for a cron expression
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows/cron/preview",
    tag = "workflows",
    params(("expr" = String, Query, description = "Cron expression")),
    responses(
        (status = 200, description = "Preview next run times", body = [String]),
        (status = 422, description = "Invalid cron expression")
    ),
    security(("jwt" = []))
)]
#[get("/cron/preview")]
pub async fn cron_preview(
    query: web::Query<std::collections::HashMap<String, String>>,
    _: RequiredAuth,
) -> impl Responder {
    let expr = match query.get("expr") {
        Some(v) if !v.trim().is_empty() => v.clone(),
        _ => return ApiResponse::<()>::unprocessable_entity("Missing expr parameter"),
    };

    match utils::preview_next(&expr, 5) {
        Ok(next) => ApiResponse::ok(next),
        Err(e) => ApiResponse::<()>::unprocessable_entity(&format!("Invalid cron: {}", e)),
    }
}
