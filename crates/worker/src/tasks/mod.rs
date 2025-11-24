#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod version_purger;

pub use version_purger::VersionPurgerTask;

