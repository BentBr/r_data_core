use anyhow::Context;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn get_provider_config(
    pool: &PgPool,
    uuid: Uuid,
) -> anyhow::Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        "SELECT config FROM workflows WHERE uuid = $1 AND kind = 'provider'::workflow_kind AND enabled = true",
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
    .context("select provider config")?;

    let cfg = row.map(|r| r.get::<serde_json::Value, _>("config"));
    Ok(cfg)
}
