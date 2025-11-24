#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod entity_versioning;
pub mod keys;

pub use entity_versioning::EntityVersioningSettings;
pub use keys::SystemSettingKey;

