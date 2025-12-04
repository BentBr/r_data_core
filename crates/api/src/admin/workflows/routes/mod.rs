#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod cron;
pub mod crud;
pub mod list;
pub mod runs;
pub mod utils;
pub mod versions;

use actix_web::web;

/// Register workflow routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list::list_workflows)
        // Register static 'runs' routes BEFORE dynamic '/{uuid}' to avoid conflicts
        .service(list::list_all_workflow_runs)
        .service(cron::cron_preview)
        .service(runs::run_workflow_now_upload)
        .service(runs::list_workflow_run_logs)
        .service(list::list_workflow_runs)
        // Dynamic UUID routes
        .service(crud::get_workflow_details)
        .service(crud::create_workflow)
        .service(crud::update_workflow)
        .service(crud::delete_workflow)
        .service(runs::run_workflow_now)
        .service(versions::list_workflow_versions)
        .service(versions::get_workflow_version);
}
