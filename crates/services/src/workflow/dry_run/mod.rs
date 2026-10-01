#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Executing a DSL program without changing anything.
//!
//! The dry-run runs the *real* step executor rather than a simulation. That
//! matters because `DslProgram::execute` — which looks like a ready-made pure
//! preview — skips the four async transforms, so a trace built from it would
//! be confidently wrong for exactly the programs people most need help with.
//!
//! Safety comes from two mechanisms, chosen per effect according to whether it
//! could be undone:
//!
//! * **Entity reads and writes** go through [`DryRunEntityRepository`], which
//!   reads the live database and writes only to memory. Read-your-writes holds,
//!   so a step that creates an entity and a later step that resolves it behave
//!   as they would in production.
//! * **Email and outbound pushes** are suppressed outright and reported. No
//!   overlay can un-send an email, so they are never attempted.

mod repository;
#[cfg(test)]
mod repository_tests;
mod service;

pub use repository::{DryRunEntityRepository, RecordedWrite, WriteKind};
pub use service::{execute_dry_run, DryRunDeps};
