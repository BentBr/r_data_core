#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod runner;
mod services;
mod state;

pub(crate) use runner::spawn_consumer_loop;
