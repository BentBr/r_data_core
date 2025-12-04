#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_core::public_api::{BrowseKind, BrowseNode};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(sqlx::FromRow, Clone)]
struct RowRec {
    uuid: Uuid,
    entity_type: String,
    path: String,
    entity_key: String,
}

/// Browse dynamic entities by virtual path
///
/// Returns folders (first) and files directly under the path, representing the hierarchical
/// structure of dynamic entities in the `entities_registry`.
///
/// * `pool` - `PostgreSQL` connection ``PgPool``
///
/// # Errors
/// Returns an error if the database query fails
#[allow(clippy::too_many_lines)]
pub async fn browse_by_path(
    db_pool: &PgPool,
    raw_path: &str,
    limit: i64,
    offset: i64,
) -> Result<(Vec<BrowseNode>, i64)> {
    // Normalize input path
    let prefix = normalize_path(raw_path);

    // Query all paths at or below prefix (single round-trip)
    let rows = query_paths(db_pool, &prefix).await?;

    // Map of exact paths + entity_key to entity info
    let exact = build_exact_path_map(&rows);

    // Build first-level folders and files
    let base_len = if prefix == "/" { 1 } else { prefix.len() + 1 };
    let (folder_map, files, _file_names) =
        build_files_and_folders(&rows, &prefix, base_len, &exact);

    // Sort folders and files alphabetically by name (case-insensitive)
    let mut all = sort_and_combine(folder_map, files);

    // Check for children on files and folders
    check_children(db_pool, &mut all).await?;

    // Paginate results
    #[allow(clippy::cast_possible_wrap)]
    let total = all.len() as i64;
    let page = paginate_results(&all, offset, limit);

    Ok((page, total))
}

fn normalize_path(raw_path: &str) -> String {
    let mut prefix = raw_path.to_string();
    if prefix.is_empty() {
        prefix = "/".to_string();
    }
    if !prefix.starts_with('/') {
        prefix = format!("/{prefix}");
    }
    if prefix.len() > 1 {
        prefix = prefix.trim_end_matches('/').to_string();
    }
    prefix
}

async fn query_paths(db_pool: &PgPool, prefix: &str) -> Result<Vec<RowRec>> {
    if prefix == "/" {
        sqlx::query_as::<_, RowRec>(
            "SELECT uuid, entity_type, path, entity_key FROM entities_registry WHERE path = '/' OR path LIKE '/%'",
        )
        .fetch_all(db_pool)
        .await
        .map_err(Into::into)
    } else {
        sqlx::query_as::<_, RowRec>(
            "SELECT uuid, entity_type, path, entity_key FROM entities_registry WHERE path = $1 OR path LIKE $1 || '/%'",
        )
        .bind(prefix)
        .fetch_all(db_pool)
        .await
        .map_err(Into::into)
    }
}

fn build_exact_path_map(rows: &[RowRec]) -> HashMap<String, (Uuid, String)> {
    let mut exact: HashMap<String, (Uuid, String)> = HashMap::new();
    for r in rows {
        let key = format!("{}::{}", r.path, r.entity_key);
        exact.insert(key, (r.uuid, r.entity_type.clone()));
    }
    exact
}

fn build_files_and_folders(
    rows: &[RowRec],
    prefix: &str,
    base_len: usize,
    exact: &HashMap<String, (Uuid, String)>,
) -> (
    HashMap<String, BrowseNode>,
    Vec<BrowseNode>,
    HashSet<String>,
) {
    let mut folder_map: HashMap<String, BrowseNode> = HashMap::new();
    let mut files: Vec<BrowseNode> = Vec::new();
    let mut file_names: HashSet<String> = HashSet::new();

    // First pass: add files whose row.path equals the requested prefix
    for r in rows {
        let p = r.path.as_str();
        if p == prefix {
            let entity_key = r.entity_key.clone();
            let exact_key = format!("{p}::{entity_key}");
            let (entity_uuid, entity_type) = exact
                .get(&exact_key)
                .cloned()
                .map_or((None, None), |(u, t)| (Some(u), Some(t)));

            // Child folder path for this file (so FE can lazy-load its children by path)
            let child_path = if p == "/" {
                format!("/{entity_key}")
            } else {
                format!("{p}/{entity_key}")
            };

            files.push(BrowseNode {
                kind: BrowseKind::File,
                name: entity_key.clone(),
                path: child_path,
                entity_uuid,
                entity_type,
                has_children: Some(false),
            });

            file_names.insert(entity_key);
        }
    }

    // Second pass: add first-level folders (unique segment after prefix),
    // but skip folders whose name matches an existing file at this level
    for r in rows {
        let p = r.path.as_str();
        if p == prefix {
            continue;
        }

        // Ensure this path is deeper than the prefix
        if p.len() <= base_len {
            continue;
        }

        let remainder = &p[base_len..];
        let seg = match remainder.split('/').next() {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };

        if file_names.contains(seg) {
            continue;
        }

        let folder_path = if prefix == "/" {
            format!("/{seg}")
        } else {
            format!("{prefix}/{seg}")
        };

        folder_map
            .entry(seg.to_string())
            .or_insert_with(|| BrowseNode {
                kind: BrowseKind::Folder,
                name: seg.to_string(),
                path: folder_path,
                entity_uuid: None,
                entity_type: None,
                has_children: Some(true),
            });
    }

    (folder_map, files, file_names)
}

fn sort_and_combine(
    folder_map: HashMap<String, BrowseNode>,
    mut files: Vec<BrowseNode>,
) -> Vec<BrowseNode> {
    let mut folders: Vec<BrowseNode> = folder_map.into_values().collect();
    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut all = Vec::new();
    all.extend(folders);
    all.extend(files);
    all
}

async fn check_children(db_pool: &PgPool, nodes: &mut [BrowseNode]) -> Result<()> {
    for node in nodes.iter_mut() {
        if let Some(uuid) = node.entity_uuid {
            if node.has_children == Some(true) {
                // Already marked as having children
                continue;
            }
            // Check if this entity has children by querying for entities with this as parent
            let has_children_result = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM entities_registry WHERE parent_uuid = $1 LIMIT 1)",
            )
            .bind(uuid)
            .fetch_one(db_pool)
            .await;

            if let Ok(has) = has_children_result {
                node.has_children = Some(has);
            }
        } else if node.kind == BrowseKind::Folder {
            // For folders (virtual), check if there are entities at this path or below
            let path = &node.path;
            let has_children_result = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM entities_registry WHERE path = $1 OR path LIKE $1 || '/%' LIMIT 1)"
            )
            .bind(path)
            .fetch_one(db_pool)
            .await;

            if let Ok(has) = has_children_result {
                node.has_children = Some(has);
            }
        }
    }
    Ok(())
}

fn paginate_results(all: &[BrowseNode], offset: i64, limit: i64) -> Vec<BrowseNode> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let start = offset.max(0) as usize;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let end = (offset + limit).max(0) as usize;
    if start >= all.len() {
        vec![]
    } else {
        all[start..all.len().min(end)].to_vec()
    }
}
