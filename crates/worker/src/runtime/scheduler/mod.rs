#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod jobs;
mod reconcile;
mod startup;

pub(crate) use startup::start_scheduler;
