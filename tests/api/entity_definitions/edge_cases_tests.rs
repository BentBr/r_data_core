#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for edge cases in entity definition constraints.

use super::common::{create_test_app, create_test_jwt_token};
use actix_web::{
    http::{header, StatusCode},
    test,
};
use r_data_core_core::error::Result;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;

/// Test that null constraint values are handled correctly.
#[tokio::test]
#[serial]
async fn test_null_constraint_values_handled_correctly() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    // Create with null constraint values (as sent by frontend when clearing values)
    let payload = serde_json::json!({
        "entity_type": "test_nullable",
        "display_name": "Test Nullable",
        "description": "Entity with null constraints",
        "group_name": "test",
        "allow_children": false,
        "icon": "file",
        "fields": [
            {
                "name": "description",
                "display_name": "Description",
                "field_type": "String",
                "required": false,
                "indexed": false,
                "filterable": false,
                "unique": false,
                "constraints": {
                    "type": "string",
                    "constraints": {
                        "pattern": null,
                        "min_length": null,
                        "max_length": null
                    }
                }
            }
        ],
        "published": true
    });

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/entity-definitions")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Should handle null constraint values"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test that entity definition without constraints works correctly.
#[tokio::test]
#[serial]
async fn test_entity_definition_without_constraints() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    let payload = serde_json::json!({
        "entity_type": "test_simple",
        "display_name": "Test Simple",
        "description": "Simple entity without constraints",
        "group_name": "test",
        "allow_children": false,
        "icon": "file",
        "fields": [
            {
                "name": "title",
                "display_name": "Title",
                "field_type": "String",
                "required": true,
                "indexed": true,
                "filterable": true
            },
            {
                "name": "content",
                "display_name": "Content",
                "field_type": "Text",
                "required": false,
                "indexed": false,
                "filterable": false
            }
        ],
        "published": true
    });

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/entity-definitions")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Entity without constraints should be created successfully"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "Success");

    clear_test_db(&pool).await?;
    Ok(())
}
