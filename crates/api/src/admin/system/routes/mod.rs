#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod info;
pub mod settings;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use info::*;
pub use settings::*;

/// Register system routes
pub fn register_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(get_entity_versioning_settings);
    cfg.service(update_entity_versioning_settings);
    cfg.service(get_workflow_run_log_settings);
    cfg.service(update_workflow_run_log_settings);
    cfg.service(get_outbox_settings);
    cfg.service(update_outbox_settings);
    cfg.service(get_license_status);
    cfg.service(get_system_versions);
    cfg.service(get_capabilities);
    cfg.service(list_system_logs);
    cfg.service(get_system_log);
    // Internal endpoint (not in Swagger)
    cfg.service(verify_license_internal);
}
