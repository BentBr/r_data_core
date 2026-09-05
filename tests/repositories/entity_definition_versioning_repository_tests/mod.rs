#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod prune_tests;
pub mod snapshot_and_list_tests;
pub mod version_and_metadata_tests;

use r_data_core_test_support::{unique_entity_type, TestDatabase};
use sqlx::PgPool;
use uuid::Uuid;

/// Seed a minimal entity definition satisfying FK constraints.
/// Returns the definition UUID.
///
/// # Panics
/// Panics if the database INSERT fails.
pub async fn seed_entity_definition(pool: &PgPool, creator: Uuid) -> Uuid {
    let uuid = Uuid::now_v7();
    let entity_type = unique_entity_type("entver");
    sqlx::query(
        "INSERT INTO entity_definitions
             (uuid, entity_type, display_name, created_by, field_definitions, version)
         VALUES ($1, $2, $3, $4, '[]'::jsonb, 1)",
    )
    .bind(uuid)
    .bind(entity_type)
    .bind("Versioning Test Entity")
    .bind(creator)
    .execute(pool)
    .await
    .expect("failed to seed entity_definition");
    uuid
}

/// Bump the `version` column so the next snapshot records a new version number.
///
/// # Panics
/// Panics if the database UPDATE fails.
pub async fn bump_definition_version(pool: &PgPool, def_uuid: Uuid, updated_by: Uuid) {
    sqlx::query(
        "UPDATE entity_definitions
         SET version = version + 1, updated_by = $2
         WHERE uuid = $1",
    )
    .bind(def_uuid)
    .bind(updated_by)
    .execute(pool)
    .await
    .expect("failed to bump entity_definition version");
}

/// Convenience wrapper returned by the per-module setup helper.
pub struct TestContext {
    pub db: TestDatabase,
}
