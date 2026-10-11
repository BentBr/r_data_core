#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::expect_used)]
//! Response-body hygiene.
//!
//! A 500 that interpolates the underlying error hands the caller whatever the
//! database or a driver put in its message — table names, constraint names,
//! sometimes values. The pattern is easy to write and hard to see in review,
//! so it is forbidden mechanically: log the error, return a generic message.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: &str = "internal_error(&format!";

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable directory") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn internal_errors_never_interpolate_the_underlying_error() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/api/src");
    let mut files = Vec::new();
    rust_files(&root, &mut files);

    let mut hits = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("readable file");
        for (index, line) in text.lines().enumerate() {
            if line.contains(FORBIDDEN) {
                hits.push(format!("{}:{}", file.display(), index + 1));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "log the error and return a generic message instead of `{FORBIDDEN}`:\n{}",
        hits.join("\n")
    );
}
