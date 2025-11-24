#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod dynamic_entity;

pub mod abstract_entity;

pub use abstract_entity::{AbstractRDataEntity, DynamicFields};

