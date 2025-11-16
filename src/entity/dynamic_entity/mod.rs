//! Dynamic entity module provides interfaces for working with dynamic data

pub mod entity;
pub mod mapper;
pub mod repository;
pub mod repository_trait;
pub mod repository_update;
pub mod utils;
pub mod validator;
pub mod versioning;

pub use entity::{DynamicEntity, FromValue, ToValue};
pub use repository::DynamicEntityRepository;
pub use repository_trait::DynamicEntityRepositoryTrait;
pub use validator::{validate_entity, validate_entity_with_violations, FieldViolation};
