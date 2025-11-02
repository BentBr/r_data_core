use anyhow::Context;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn get_provider_config(
    pool: &PgPool,
    uuid: Uuid,
) -> anyhow::Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        r#"SELECT provider_config FROM workflows WHERE uuid = $1 AND kind = 'provider'::data_workflow_kind AND enabled = true"#,
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
    .context("select provider_config")?;

    let cfg = row
        .map(|r| r.get::<Option<serde_json::Value>, _>("provider_config"))
        .flatten();
    Ok(cfg)
}
