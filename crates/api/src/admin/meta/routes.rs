#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, web, Responder};
use log::error;

use crate::admin::meta::models::DashboardStats;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::RequiredAuthExt;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};

/// Get dashboard statistics
///
/// Returns aggregated statistics for the dashboard including:
/// - Entity definitions count
/// - Entities count (total and by type)
/// - Workflows count and latest run statuses
/// - Online users count
#[utoipa::path(
    get,
    path = "/admin/api/v1/meta/dashboard",
    tag = "meta",
    responses(
        (status = 200, description = "Dashboard statistics", body = DashboardStats),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/dashboard")]
pub async fn get_dashboard_stats(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission - need DashboardStats:Read to view dashboard statistics
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::DashboardStats,
        &PermissionType::Read,
        None,
    ) {
        return resp;
    }

    let service = state.get_ref().dashboard_stats_service();

    match service.get_dashboard_stats().await {
        Ok(repo_stats) => ApiResponse::ok(DashboardStats::from(repo_stats)),
        Err(e) => {
            error!("Failed to get dashboard stats: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve dashboard statistics")
        }
    }
}

/// Register meta routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_dashboard_stats);
}
