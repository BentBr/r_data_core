#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;
use std::fs::read_to_string;

/// Load an example JSON file from `.example_files/json_examples/dsl/`
pub fn load_example(path: &str) -> Value {
    let content =
        read_to_string(format!(".example_files/json_examples/dsl/{path}")).expect("read example");
    serde_json::from_str(&content).expect("parse json")
}

/// Load a test fixture JSON file from `.example_files/json_examples/dsl/tests/`
pub fn load_test_fixture(path: &str) -> Value {
    let content = read_to_string(format!(".example_files/json_examples/dsl/tests/{path}"))
        .expect("read test fixture");
    serde_json::from_str(&content).expect("parse json")
}

pub mod casting_invalid_tests;
pub mod casting_tests;
pub mod chaining_tests;
pub mod edge_case_tests;
pub mod fanout_tests;
pub mod mapping_tests;
pub mod validation_tests;
