use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use crate::VersionRepository;
use r_data_core_core::error::Result;

/// Create a pre-update snapshot for a dynamic entity into `entities_versions`.
///
/// This function MUST be called within a transaction before the version is incremented.
/// The snapshot's `created_by` is set to the current `updated_by` (or `created_by` if `updated_by` is `None`).
///
/// # Errors
/// Returns an error if the database operation fails
pub async fn snapshot_pre_update(
    tx: &mut Transaction<'_, Postgres>,
    uuid: Uuid,
    _new_updated_by: Option<Uuid>, // Not used - extracted from entities_registry
) -> Result<()> {
    VersionRepository::snapshot_pre_update_tx(tx, uuid)
        .await
        .map_err(|e| r_data_core_core::error::Error::Unknown(e.to_string()))
}
