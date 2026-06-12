#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Integration tests for the generic `EntityRepository<T>` and
//! `PgPoolExtension` in `crates/persistence/src/repository.rs`.
//!
//! `EntityRepository::create` and `EntityRepository::update` bind timestamp
//! fields as `&str` (text) but real tables have `TIMESTAMPTZ` columns.
//! Postgres rejects implicit text→timestamptz casts for parameterized queries
//! (error 42804).  The BUG is documented via `test_create_returns_type_error`.
//!
//! All other methods (`get_by_uuid`, `list`, `count`, `delete`,
//! `get_version`, `list_versions`) are tested by seeding rows with raw SQL.

pub mod crud_tests;
pub mod versioning_tests;

pub use r_data_core_core::error::Result;
pub use r_data_core_persistence::{EntityRepository, PgPoolExtension};
pub use r_data_core_test_support::setup_test_db;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
pub use uuid::Uuid;

// ---------------------------------------------------------------------------
// Minimal test struct
// ---------------------------------------------------------------------------

/// Columns returned by `SELECT *` on `test_items`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TestItem {
    pub uuid: Uuid,
    pub path: String,
    pub published: bool,
    pub version: i64,
    pub custom_fields: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Per-test helpers
// ---------------------------------------------------------------------------

/// # Panics
///
/// Panics if the `CREATE TABLE` query fails.
pub async fn create_test_table(pool: &sqlx::PgPool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS test_items (
            uuid          UUID PRIMARY KEY DEFAULT uuidv7(),
            path          TEXT NOT NULL,
            created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            created_by    UUID,
            updated_by    UUID,
            published     BOOLEAN NOT NULL DEFAULT FALSE,
            version       BIGINT NOT NULL DEFAULT 1,
            custom_fields JSONB NOT NULL DEFAULT '{}'::jsonb
        )",
    )
    .execute(pool)
    .await
    .expect("create test_items");
}

/// Insert one row via raw SQL and return its UUID.
///
/// # Panics
///
/// Panics if the `INSERT` query fails.
pub async fn seed_item(pool: &sqlx::PgPool, path: &str, published: bool) -> Uuid {
    let uuid = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO test_items (uuid, path, published, version)
         VALUES ($1, $2, $3, 1)",
    )
    .bind(uuid)
    .bind(path)
    .bind(published)
    .execute(pool)
    .await
    .expect("seed_item");
    uuid
}

#[must_use]
pub fn repo(pool: &sqlx::PgPool) -> EntityRepository<TestItem> {
    pool.repository_with_table::<TestItem>("test_items")
}
