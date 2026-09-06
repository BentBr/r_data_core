#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // actix test utilities are Rc-based

//! MCP server integration tests, against a real `RDataCore` on a real port.

pub mod harness;
pub mod round_trip_tests;
