#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod diagnostics;
pub mod handlers;
pub mod options_builders;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use handlers::*;

pub fn register_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("")
            .service(validate_dsl)
            .service(list_from_options)
            .service(list_to_options)
            .service(list_transform_options),
    );
}
