mod notification;
pub mod refresh_token;
// mod workflow_definition;
// mod workflow_task;

// Dynamic entity validator moved to r_data_core_core::domain::dynamic_entity::validator
// pub mod dynamic_entity; // Removed - validator migrated to core crate

// Re-export from core
pub use r_data_core_core::domain::AbstractRDataEntity;
