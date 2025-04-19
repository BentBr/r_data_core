use actix_web::{get, HttpRequest, HttpResponse, Responder};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::api::models::HealthData;

/// Common health check handler for both admin and public API routes
pub async fn health_check_handler(req: HttpRequest) -> impl Responder {
    // Generate UUID for this health check
    let health_id = Uuid::now_v7();

    // Extract path and user agent
    let path = req.path().to_string();
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    // Create health data
    let health_data = HealthData {
        date: OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
        uuid: health_id,
        route: path,
        agent: user_agent,
    };

    // Return response with the custom message 'Service healthy'
    HttpResponse::Ok().json(serde_json::json!({
        "status": "Success",
        "message": "Service healthy",
        "data": health_data
    }))
}

/// Admin API health check endpoint
#[utoipa::path(
    get,
    path = "/admin/api/v1/health",
    tag = "admin-health",
    responses(
        (status = 200, description = "Health check successful", body = HealthData),
    ),
    security()
)]
#[get("/admin/api/v1/health")]
pub async fn admin_health_check(req: HttpRequest) -> impl Responder {
    health_check_handler(req).await
}

/// Public API health check endpoint
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "public-health",
    responses(
        (status = 200, description = "Health check successful", body = HealthData),
    ),
    security()
)]
#[get("/api/v1/health")]
pub async fn public_health_check(req: HttpRequest) -> impl Responder {
    health_check_handler(req).await
}
