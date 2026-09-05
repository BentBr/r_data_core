#![deny(unsafe_code)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

pub mod data;
pub mod dsl;
