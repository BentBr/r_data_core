#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::public::entities::models::{BrowseKind, BrowseNode, EntityTypeInfo};
use r_data_core_core::error::Result;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct EntityRepository {
    db_pool: PgPool,
}

impl EntityRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn list_available_entities(&self) -> Result<Vec<EntityTypeInfo>> {
        let rows = sqlx::query!(
            "
            SELECT entity_type as name, display_name, description,
                   uuid as entity_definition_uuid
            FROM entity_definitions
            WHERE published = true
            ",
        )
        .fetch_all(&self.db_pool)
        .await?;

        let mut result = Vec::new();

        for row in rows {
            // Count entity instances
            let entity_count = get_entity_count(&self.db_pool, &row.name)
                .await
                .unwrap_or(0);

            // Count fields for each entity
            let field_count: i64 = match sqlx::query_scalar!(
                "
                SELECT COUNT(*) as count
                FROM entity_definitions
                WHERE entity_type = $1
                ",
                row.name
            )
            .fetch_one(&self.db_pool)
            .await
            {
                Ok(count) => count.unwrap_or(0),
                Err(_) => 0,
            };

            result.push(EntityTypeInfo {
                name: row.name,
                display_name: row.display_name,
                description: row.description,
                is_system: false,
                entity_count,
                field_count: field_count as i32,
            });
        }

        Ok(result)
    }

    /// Browse the registry by virtual path. Returns folders (first) and files directly under the path.
    pub async fn browse_by_path(
        &self,
        raw_path: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<BrowseNode>, i64)> {
        // Normalize input path
        let mut prefix = raw_path.to_string();
        if prefix.is_empty() {
            prefix = "/".to_string();
        }
        if !prefix.starts_with('/') {
            prefix = format!("/{}", prefix);
        }
        if prefix.len() > 1 {
            prefix = prefix.trim_end_matches('/').to_string();
        }

        // Query all paths at or below prefix (single round-trip)
        #[derive(sqlx::FromRow, Clone)]
        struct RowRec {
            uuid: Uuid,
            entity_type: String,
            path: String,
            entity_key: String,
        }

        let rows: Vec<RowRec> = if prefix == "/" {
            sqlx::query_as::<_, RowRec>(
                "SELECT uuid, entity_type, path, entity_key FROM entities_registry WHERE path = '/' OR path LIKE '/%'",
            )
            .fetch_all(&self.db_pool)
            .await?
        } else {
            sqlx::query_as::<_, RowRec>(
                "SELECT uuid, entity_type, path, entity_key FROM entities_registry WHERE path = $1 OR path LIKE $1 || '/%'",
            )
            .bind(&prefix)
            .fetch_all(&self.db_pool)
            .await?
        };

        // Map of exact paths + entity_key to entity info
        // Use path + entity_key as the key to handle multiple entities at the same path
        let mut exact: HashMap<String, (Uuid, String)> = HashMap::new();
        for r in &rows {
            let key = format!("{}::{}", r.path, r.entity_key);
            exact.insert(key, (r.uuid, r.entity_type.clone()));
        }

        // Build first-level folders and files
        let base_len = if prefix == "/" { 1 } else { prefix.len() + 1 };
        let mut folder_map: HashMap<String, BrowseNode> = HashMap::new();
        let mut files: Vec<BrowseNode> = Vec::new();
        let mut has_children: HashSet<String> = HashSet::new();

        // Track names of files that live directly under the requested prefix
        let mut file_names: HashSet<String> = HashSet::new();

        // First pass: add files whose row.path equals the requested prefix
        for r in &rows {
            let p = r.path.as_str();
            if p == prefix {
                let entity_key = r.entity_key.clone();
                let exact_key = format!("{}::{}", p, &entity_key);
                let (entity_uuid, entity_type) = exact
                    .get(&exact_key)
                    .cloned()
                    .map(|(u, t)| (Some(u), Some(t)))
                    .unwrap_or((None, None));

                // Child folder path for this file (so FE can lazy-load its children by path)
                let child_path = if p == "/" {
                    format!("/{}", entity_key)
                } else {
                    format!("{}/{}", p, entity_key)
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
        for r in &rows {
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
                format!("/{}", seg)
            } else {
                format!("{}/{}", prefix, seg)
            };

            has_children.insert(folder_path.clone());
            folder_map.entry(seg.to_string()).or_insert(BrowseNode {
                kind: BrowseKind::Folder,
                name: seg.to_string(),
                path: folder_path,
                entity_uuid: None,
                entity_type: None,
                has_children: Some(true),
            });
        }

        // Sort folders and files alphabetically by name (case-insensitive)
        let mut folders: Vec<BrowseNode> = folder_map.into_values().collect();
        folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let mut all = Vec::new();
        all.extend(folders);
        all.extend(files);

        // Check for children on files (entities with parent_uuid)
        for node in &mut all {
            if let Some(uuid) = node.entity_uuid {
                if let Some(true) = node.has_children {
                    // Already marked as having children
                    continue;
                }
                // Check if this entity has children by querying for entities with this as parent
                let has_children_result = sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS(SELECT 1 FROM entities_registry WHERE parent_uuid = $1 LIMIT 1)",
                )
                .bind(uuid)
                .fetch_one(&self.db_pool)
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
                .fetch_one(&self.db_pool)
                .await;

                if let Ok(has) = has_children_result {
                    node.has_children = Some(has);
                }
            }
        }

        let total = all.len() as i64;
        let start = offset.max(0) as usize;
        let end = (offset + limit).max(0) as usize;
        let page = if start >= all.len() {
            vec![]
        } else {
            all[start..all.len().min(end)].to_vec()
        };

        Ok((page, total))
    }
}

async fn get_entity_count(pool: &PgPool, entity_type: &str) -> Result<i64> {
    let table_name = format!("{}_entities", entity_type.to_lowercase());

    // Check if table exists first
    let table_exists: bool = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_schema = 'public'
            AND table_name = $1
        ) as "exists!"
        "#,
        table_name
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        return Ok(0);
    }

    let query = format!("SELECT COUNT(*) FROM \"{}\"", table_name);
    let count: i64 = sqlx::query_scalar(&query).fetch_one(pool).await?;

    Ok(count)
}

