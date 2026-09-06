#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
//! Architecture / layering enforcement.
//!
//! Encodes the workspace crate-dependency invariants documented in the
//! `architecture` skill and fails CI when a layering rule is broken — so a
//! forbidden edge (e.g. `core` reaching up into `persistence`, or anything
//! depending on `api`) is caught here instead of in review.
//!
//! The rules are expressed as an **allowlist**: each crate's set of internal
//! (`r_data_core_*`) compile-time dependencies must be a subset of what is
//! permitted below. Adding a new cross-crate edge therefore requires a
//! deliberate update to `allowed_internal_dependencies`, which is the point.

use std::collections::{BTreeSet, HashMap};
use std::process::Command;

use serde_json::Value;

/// The application entrypoint package; it wires every layer together and is
/// exempt from the layering allowlist.
const ROOT_PACKAGE: &str = "r_data_core";

/// Permitted direct internal dependencies per workspace crate.
///
/// `core` is the domain leaf (no internal deps). Nothing may depend on `api`
/// or `worker` — they sit at the top of the stack. `workflow` is a low-level
/// engine that may only reach `core`.
fn allowed_internal_dependencies() -> HashMap<&'static str, BTreeSet<&'static str>> {
    let rules: &[(&str, &[&str])] = &[
        ("r_data_core_core", &[]),
        ("r_data_core_license", &["r_data_core_core"]),
        ("r_data_core_workflow", &["r_data_core_core"]),
        (
            "r_data_core_persistence",
            &["r_data_core_core", "r_data_core_workflow"],
        ),
        (
            "r_data_core_services",
            &[
                "r_data_core_core",
                "r_data_core_license",
                "r_data_core_persistence",
                "r_data_core_workflow",
            ],
        ),
        (
            "r_data_core_worker",
            &[
                "r_data_core_core",
                "r_data_core_persistence",
                "r_data_core_services",
                "r_data_core_workflow",
            ],
        ),
        (
            "r_data_core_mcp",
            &["r_data_core_core", "r_data_core_workflow"],
        ),
        (
            "r_data_core_api",
            &[
                "r_data_core_core",
                "r_data_core_license",
                "r_data_core_persistence",
                "r_data_core_services",
                "r_data_core_workflow",
            ],
        ),
        (
            "r_data_core_test_support",
            &[
                "r_data_core_core",
                "r_data_core_persistence",
                "r_data_core_services",
                "r_data_core_workflow",
            ],
        ),
    ];
    rules
        .iter()
        .map(|(k, v)| (*k, v.iter().copied().collect()))
        .collect()
}

/// Map each workspace crate to the set of internal (`r_data_core_*`) crates it
/// depends on at **compile time** (normal deps only; dev/build deps are
/// excluded so test-only helpers don't count as layering edges).
fn internal_dependency_graph() -> HashMap<String, BTreeSet<String>> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("failed to run `cargo metadata`");
    assert!(
        output.status.success(),
        "`cargo metadata` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let meta: Value = serde_json::from_slice(&output.stdout).expect("invalid cargo metadata JSON");
    let packages = meta["packages"]
        .as_array()
        .expect("metadata.packages is an array");

    let mut graph = HashMap::new();
    for pkg in packages {
        let name = pkg["name"].as_str().expect("package name").to_string();
        if !name.starts_with("r_data_core") {
            continue;
        }
        let deps: BTreeSet<String> = pkg["dependencies"]
            .as_array()
            .expect("package dependencies is an array")
            .iter()
            // `kind` is null for normal deps, "dev"/"build" otherwise.
            .filter(|d| d.get("kind").is_none_or(Value::is_null))
            .filter_map(|d| d["name"].as_str())
            .filter(|d| d.starts_with("r_data_core") && *d != name)
            .map(String::from)
            .collect();
        graph.insert(name, deps);
    }
    graph
}

#[test]
fn core_is_a_dependency_free_leaf() {
    let graph = internal_dependency_graph();
    let core = graph
        .get("r_data_core_core")
        .expect("core crate present in workspace metadata");
    assert!(
        core.is_empty(),
        "core must not depend on any internal crate (domain purity); found: {core:?}"
    );
}

#[test]
fn nothing_depends_on_top_layer_crates() {
    let graph = internal_dependency_graph();
    for top in ["r_data_core_api", "r_data_core_worker"] {
        for (crate_name, deps) in &graph {
            if crate_name == ROOT_PACKAGE {
                continue;
            }
            assert!(
                !deps.contains(top),
                "{crate_name} must not depend on {top}: it sits at the top of the stack"
            );
        }
    }
}

#[test]
fn internal_dependencies_respect_the_layering_allowlist() {
    let allowed = allowed_internal_dependencies();
    let graph = internal_dependency_graph();
    let mut violations = Vec::new();

    for (crate_name, deps) in &graph {
        if crate_name == ROOT_PACKAGE {
            continue; // the application entrypoint may wire every layer
        }
        let Some(permitted) = allowed.get(crate_name.as_str()) else {
            violations.push(format!(
                "{crate_name} is not in the layering allowlist — add it to `allowed_internal_dependencies`"
            ));
            continue;
        };
        for dep in deps {
            if !permitted.contains(dep.as_str()) {
                violations.push(format!(
                    "forbidden layering edge: {crate_name} -> {dep} (not in the allowlist)"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "crate layering violations:\n{}",
        violations.join("\n")
    );
}
