#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::email_template::EmailTemplateType;
use r_data_core_core::error::Result;
use r_data_core_persistence::{EmailTemplateRepository, EmailTemplateRepositoryTrait};
use r_data_core_test_support::{clear_test_db, setup_test_db};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_email_template_crud() -> Result<()> {
    let pool = setup_test_db().await;
    // Do NOT clear the DB here — we need the seeded system templates from migrations.
    // Instead we read them and then create a distinct workflow template.

    let repo = EmailTemplateRepository::new(pool.pool.clone());

    // System template should exist from migration seed
    let system_templates = repo.list_by_type(EmailTemplateType::System).await?;
    assert!(
        !system_templates.is_empty(),
        "Expected at least one seeded system template"
    );
    assert!(
        system_templates.iter().any(|t| t.slug == "password_reset"),
        "Expected seeded password_reset system template"
    );

    // Create a workflow template using a unique slug to avoid conflicts
    let unique_id = Uuid::now_v7();
    let slug = format!("test_workflow_{}", unique_id.simple());
    let created_by = Uuid::now_v7();

    let uuid = repo
        .create(
            "Test Template",
            &slug,
            EmailTemplateType::Workflow,
            "Subject {{name}}",
            "<p>Hello {{name}}</p>",
            "Hello {{name}}",
            serde_json::json!([{"key": "name", "description": "User name"}]),
            created_by,
        )
        .await?;

    // Read back by UUID
    let template = repo
        .get_by_uuid(uuid)
        .await?
        .expect("template should exist after creation");
    assert_eq!(template.name, "Test Template");
    assert_eq!(template.slug, slug);
    assert_eq!(template.template_type, EmailTemplateType::Workflow);
    assert_eq!(template.subject_template, "Subject {{name}}");

    // Get by slug
    let by_slug = repo
        .get_by_slug(&slug)
        .await?
        .expect("template should be findable by slug");
    assert_eq!(by_slug.uuid, uuid);

    // Update
    repo.update(
        uuid,
        Some("Updated Name"),
        "New Subject",
        "<p>New</p>",
        "New",
        serde_json::json!([]),
        Uuid::now_v7(),
    )
    .await?;

    let updated = repo
        .get_by_uuid(uuid)
        .await?
        .expect("template should exist after update");
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.subject_template, "New Subject");

    // Delete
    repo.delete(uuid).await?;
    assert!(
        repo.get_by_uuid(uuid).await?.is_none(),
        "template should not exist after delete"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_system_template_loaded_by_slug() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = EmailTemplateRepository::new(pool.pool.clone());

    let template = repo.get_by_slug("password_reset").await?;
    assert!(
        template.is_some(),
        "password_reset system template should be seeded by migration"
    );
    let template = template.unwrap();
    assert_eq!(template.template_type, EmailTemplateType::System);
    assert!(
        template
            .variables
            .as_array()
            .is_some_and(|arr| !arr.is_empty()),
        "password_reset template should have variables"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_email_template_list_all() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = EmailTemplateRepository::new(pool.pool.clone());

    let all = repo.list_all().await?;
    assert!(
        !all.is_empty(),
        "list_all should return at least the seeded system template"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_email_template_get_by_uuid_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = EmailTemplateRepository::new(pool.pool.clone());

    let result = repo.get_by_uuid(Uuid::now_v7()).await?;
    assert!(
        result.is_none(),
        "get_by_uuid should return None for unknown UUID"
    );

    Ok(())
}
