#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod listener;
mod recovery;

pub(crate) use recovery::spawn_outbox_recovery_loop;
