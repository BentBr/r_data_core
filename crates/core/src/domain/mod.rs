#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod dynamic_entity;

pub mod abstract_entity;

pub use abstract_entity::{AbstractRDataEntity, DynamicFields};
pub use dynamic_entity::validator::{FieldViolation, validate_entity, validate_entity_with_violations, validate_parent_path_consistency};

