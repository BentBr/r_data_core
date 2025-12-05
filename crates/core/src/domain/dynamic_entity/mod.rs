#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod entity;
pub mod validator;

pub use entity::DynamicEntity;
pub use validator::{
    validate_entity, validate_entity_with_violations, validate_parent_path_consistency,
    FieldViolation,
};
