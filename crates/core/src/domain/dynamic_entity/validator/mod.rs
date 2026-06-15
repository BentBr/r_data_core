#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod context;
mod entity_fns;
mod entity_validator;
mod field_fn;

pub use context::ValidationContext;
pub use entity_fns::{
    validate_entity, validate_entity_with_violations, validate_parent_path_consistency,
    FieldViolation,
};
pub use entity_validator::DynamicEntityValidator;
pub use field_fn::validate_field;
