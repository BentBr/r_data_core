#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::collections::HashMap;
use uuid::Uuid;

/// Compute reconcile actions between existing scheduler map (wf_id -> cron)
/// and the current set from DB (wf_id -> cron).
/// Returns (workflows_to_remove, workflows_to_add_or_update)
pub fn compute_reconcile_actions(
    existing: &HashMap<Uuid, String>,
    current: &HashMap<Uuid, String>,
) -> (Vec<Uuid>, Vec<(Uuid, String)>) {
    // Remove if missing from current or cron changed
    let mut to_remove = Vec::new();
    for (wf_id, existing_cron) in existing.iter() {
        match current.get(wf_id) {
            None => to_remove.push(*wf_id),
            Some(new_cron) if new_cron != existing_cron => to_remove.push(*wf_id),
            _ => {}
        }
    }
    // Add if new or cron changed
    let mut to_add = Vec::new();
    for (wf_id, new_cron) in current.iter() {
        match existing.get(wf_id) {
            None => to_add.push((*wf_id, new_cron.clone())),
            Some(old_cron) if old_cron != new_cron => to_add.push((*wf_id, new_cron.clone())),
            _ => {}
        }
    }
    (to_remove, to_add)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconcile_add_remove() {
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let c = Uuid::now_v7();

        let mut existing = HashMap::new();
        existing.insert(a, "*/5 * * * *".to_string());
        existing.insert(b, "0 * * * *".to_string());

        let mut current = HashMap::new();
        current.insert(a, "*/5 * * * *".to_string()); // unchanged
        current.insert(b, "*/10 * * * *".to_string()); // changed
        current.insert(c, "0 0 * * *".to_string()); // new

        let (to_remove, to_add) = compute_reconcile_actions(&existing, &current);

        assert!(to_remove.contains(&b)); // changed cron => remove old
        assert!(to_add.iter().any(|(id, _)| id == &b)); // re-add changed
        assert!(to_add.iter().any(|(id, _)| id == &c)); // add new
        assert!(!to_remove.contains(&a)); // unchanged
    }
}

