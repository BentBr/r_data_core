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
/// A hand-built 500 whose body is formatted on one of the next few lines.
const HAND_BUILT: &str = "InternalServerError()";
/// How many lines after `HAND_BUILT` the body's `format!` may sit on.
const WINDOW: usize = 3;

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

fn sources(relative: &str) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(relative),
        &mut files,
    );
    files
        .into_iter()
        .map(|file| {
            let text = fs::read_to_string(&file).expect("readable file");
            (file, text)
        })
        .collect()
}

/// `format!(..)` that interpolates an error binding named `e`.
fn formats_the_error(line: &str) -> bool {
    line.contains("format!(")
        && (line.contains("{e}") || line.contains("{e:") || line.contains(", e)"))
}

#[test]
fn internal_errors_never_interpolate_the_underlying_error() {
    let mut hits = Vec::new();
    for (file, text) in sources("crates/api/src") {
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

/// The same leak, written out by hand rather than through `internal_error`.
///
/// Scoped to the admin API: the public entity and query routes still build
/// such bodies and are a recorded follow-up. Widen this to `crates/api/src`
/// once they are fixed.
#[test]
fn hand_built_500s_never_interpolate_the_underlying_error() {
    let mut hits = Vec::new();
    for (file, text) in sources("crates/api/src/admin") {
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains(HAND_BUILT) {
                continue;
            }
            let end = (index + 1 + WINDOW).min(lines.len());
            if lines[index..end].iter().any(|l| formats_the_error(l)) {
                hits.push(format!("{}:{}", file.display(), index + 1));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "log the error and return a generic message instead of formatting it into a 500:\n{}",
        hits.join("\n")
    );
}

#[test]
fn the_hand_built_scan_recognises_the_leak_shape() {
    assert!(formats_the_error(r#"    "error": format!("Failed: {e}")"#));
    assert!(formats_the_error(
        r#"    "error": format!("Failed: {}", e)"#
    ));
    assert!(!formats_the_error(r#"    "error": "Failed""#));
    assert!(!formats_the_error(
        r#"    "error": format!("{entity_type} failed")"#
    ));
}
