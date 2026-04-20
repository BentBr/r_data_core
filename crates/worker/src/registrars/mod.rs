#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod license;
pub mod outbox_purger;
pub mod refresh_token;
pub mod statistics;
pub mod trait_;
pub mod version_purger;
pub mod workflow_run_logs_purger;

pub use license::LicenseVerificationRegistrar;
pub use outbox_purger::OutboxPurgerRegistrar;
pub use refresh_token::RefreshTokenCleanupRegistrar;
pub use statistics::StatisticsCollectionRegistrar;
pub use trait_::TaskRegistrar;
pub use version_purger::VersionPurgerRegistrar;
pub use workflow_run_logs_purger::WorkflowRunLogsPurgerRegistrar;
