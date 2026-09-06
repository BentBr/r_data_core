#![allow(clippy::expect_used)]

use super::*;

fn perms(entries: &[&str]) -> Permissions {
    Permissions {
        is_super_admin: false,
        entries: entries.iter().map(|e| (*e).to_string()).collect(),
    }
}

// ── the permission map ──────────────────────────────────────────────────────

#[test]
fn every_tool_has_a_deliberate_permission_decision() {
    // A tool absent from the map is silently ungated. That is sometimes right
    // and sometimes an oversight, so the decision is recorded per tool rather
    // than left to whether anyone remembered.
    const INTENTIONALLY_UNGATED: &[&str] = &[];

    for tool in ALL_TOOLS {
        let gated = required_permission(tool).is_some();
        let intentional = INTENTIONALLY_UNGATED.contains(tool);
        assert!(
            gated || intentional,
            "{tool} has no permission requirement. Add one, or list it in \
             INTENTIONALLY_UNGATED with a reason."
        );
    }
}

#[test]
fn permission_strings_use_the_namespaces_and_actions_the_server_issues() {
    // Namespaces come from ResourceNamespace::as_str, actions from the
    // lowercase forms in generate_workflow_permissions. A typo here hides a
    // tool from everyone and produces no error anywhere.
    const NAMESPACES: &[&str] = &["workflows", "entities", "entity_definitions"];
    const ACTIONS: &[&str] = &["read", "create", "update", "delete", "execute"];

    for tool in ALL_TOOLS {
        let Some(permission) = required_permission(tool) else {
            continue;
        };
        let (namespace, action) = permission
            .split_once(':')
            .unwrap_or_else(|| panic!("{tool}: '{permission}' is not namespace:action"));
        assert!(
            NAMESPACES.contains(&namespace),
            "{tool}: unknown namespace '{namespace}'"
        );
        assert!(
            ACTIONS.contains(&action),
            "{tool}: unknown action '{action}'"
        );
    }
}

#[test]
fn running_a_workflow_needs_execute_not_merely_update() {
    // The run endpoints check PermissionType::Execute; asking for update here
    // would offer the tool to someone the server will refuse.
    assert_eq!(
        required_permission("run_workflow"),
        Some("workflows:execute")
    );
}

#[test]
fn the_dry_run_is_gated_like_an_update_because_it_executes() {
    assert_eq!(
        required_permission("test_workflow"),
        Some("workflows:update")
    );
}

// ── visibility ──────────────────────────────────────────────────────────────

#[test]
fn a_read_only_caller_is_not_offered_write_tools() {
    let reader = perms(&["workflows:read"]);
    assert!(is_tool_visible("list_workflows", &reader));
    assert!(!is_tool_visible("create_workflow", &reader));
    assert!(!is_tool_visible("update_workflow", &reader));
    assert!(!is_tool_visible("run_workflow", &reader));
}

#[test]
fn a_caller_who_can_execute_but_not_write_gets_exactly_that() {
    let operator = perms(&["workflows:read", "workflows:execute"]);
    assert!(is_tool_visible("run_workflow", &operator));
    assert!(is_tool_visible("get_run_logs", &operator));
    assert!(!is_tool_visible("create_workflow", &operator));
}

#[test]
fn a_super_admin_sees_every_tool() {
    let admin = Permissions {
        is_super_admin: true,
        entries: Vec::new(),
    };
    assert_eq!(visible_tools(&admin).len(), ALL_TOOLS.len());
}

#[test]
fn a_caller_with_nothing_sees_no_gated_tool() {
    let visible = visible_tools(&Permissions::default());
    for tool in &visible {
        assert!(
            required_permission(tool).is_none(),
            "{tool} is gated but was offered to a caller with no permissions"
        );
    }
}

#[test]
fn entity_tools_are_gated_separately_from_workflow_tools() {
    // Someone who can author workflows cannot necessarily read customer data.
    let author = perms(&["workflows:read", "workflows:create"]);
    assert!(is_tool_visible("create_workflow", &author));
    assert!(!is_tool_visible("query_entities", &author));
    assert!(!is_tool_visible("list_entity_definitions", &author));
}

#[test]
fn a_path_scoped_entity_permission_still_offers_the_query_tool() {
    // They can read some entities; hiding the tool would be wrong, and
    // RDataCore rejects an out-of-scope path on the actual call.
    let scoped = perms(&["entities:/customers:read"]);
    assert!(is_tool_visible("query_entities", &scoped));
}

// ── the absent capability ───────────────────────────────────────────────────

#[test]
fn no_tool_deletes_anything() {
    // Deletion is prevented by the capability not existing, not by a check a
    // model could argue with. If a delete tool is ever added, this fails and
    // the decision has to be made deliberately.
    for tool in ALL_TOOLS {
        let lowered = tool.to_lowercase();
        assert!(
            !lowered.contains("delete") && !lowered.contains("remove"),
            "{tool} looks like a deletion tool; the MCP surface must not have one"
        );
    }
}

#[test]
fn the_tool_list_has_no_duplicates() {
    let mut seen: Vec<&str> = ALL_TOOLS.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "duplicate tool name in ALL_TOOLS");
}

#[test]
fn an_instance_reports_the_tools_its_caller_may_use() {
    use crate::auth::{ApiKeyBackend, CallerContext};
    use crate::client::RdcClient;
    use crate::config::Config;
    use std::collections::HashMap;
    use std::sync::Arc;

    let mut map = HashMap::new();
    map.insert(
        "RDC_BASE_URL".to_string(),
        "https://rdc.example.com".to_string(),
    );
    map.insert("RDC_API_KEY".to_string(), "secret".to_string());
    let config = Config::from_map(&map).expect("config");
    let client = RdcClient::new(
        &config,
        Arc::new(ApiKeyBackend::holding(&config, "an-admin-token").expect("backend")),
    )
    .expect("client");

    let tools = RdcTools::new(
        Arc::new(client),
        perms(&["workflows:read"]),
        CallerContext::default(),
    );

    let visible = tools.visible_tool_names();
    assert!(visible.contains(&"list_workflows"));
    assert!(
        !visible.contains(&"create_workflow"),
        "a read-only caller must not be offered a write tool: {visible:?}"
    );
}
