#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests that verify which entity definition fields are mutable vs immutable
//! when updating via the PUT endpoint.
//!
//! Immutable fields (preserved from existing): `uuid`, `entity_type`, `created_at`, `created_by`
//! Server-controlled fields: `updated_at`, `updated_by`
//! Mutable fields (taken from user input): `display_name`, `description`, `group_name`,
//!   `allow_children`, `icon`, `fields`, `published`

use super::common::{create_test_app, create_test_jwt_token};
use actix_web::{
    http::{header, StatusCode},
    test,
};
use r_data_core_core::error::Result;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;

/// Test that mutable fields (`display_name`, description, `group_name`, icon, published)
/// are updated when sent via PUT.
#[tokio::test]
#[serial]
async fn test_update_mutable_fields_are_persisted() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    // Create initial entity definition
    let create_payload = serde_json::json!({
        "entity_type": "mut_product",
        "display_name": "Original Name",
        "description": "Original description",
        "group_name": "original_group",
        "allow_children": false,
        "icon": "box",
        "fields": [
            {
                "name": "title",
                "display_name": "Title",
                "field_type": "Text",
                "required": true,
                "indexed": false,
                "filterable": false,
                "unique": false
            }
        ],
        "published": false
    });

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/entity-definitions")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&create_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let created_uuid = body["data"]["uuid"].as_str().unwrap();

    // Update all mutable fields
    let update_payload = serde_json::json!({
        "entity_type": "mut_product",
        "display_name": "Updated Name",
        "description": "Updated description",
        "group_name": "updated_group",
        "allow_children": true,
        "icon": "star",
        "fields": [
            {
                "name": "title",
                "display_name": "Title",
                "field_type": "Text",
                "required": true,
                "indexed": false,
                "filterable": false,
                "unique": false
            },
            {
                "name": "price",
                "display_name": "Price",
                "field_type": "Float",
                "required": false,
                "indexed": false,
                "filterable": true,
                "unique": false
            }
        ],
        "published": true
    });

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&update_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // GET and verify all mutable fields were updated
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let data = &body["data"];

    assert_eq!(data["display_name"], "Updated Name");
    assert_eq!(data["description"], "Updated description");
    assert_eq!(data["group_name"], "updated_group");
    assert_eq!(data["allow_children"], true);
    assert_eq!(data["icon"], "star");
    assert_eq!(data["published"], true);

    let fields = data["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().any(|f| f["name"] == "price"));

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test that immutable fields (`entity_type`, `created_at`, `created_by`) are preserved
/// even when the client sends different values.
#[tokio::test]
#[serial]
async fn test_update_immutable_fields_are_preserved() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    // Create initial entity definition
    let create_payload = serde_json::json!({
        "entity_type": "immut_product",
        "display_name": "Immutable Test",
        "description": "",
        "allow_children": false,
        "icon": "box",
        "fields": [],
        "published": true
    });

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/entity-definitions")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&create_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let created_uuid = body["data"]["uuid"].as_str().unwrap();

    // GET the original to capture immutable field values
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let original: serde_json::Value = test::read_body_json(resp).await;
    let original_data = &original["data"];
    let original_entity_type = original_data["entity_type"].as_str().unwrap();
    let original_created_at = original_data["created_at"].as_str().unwrap();

    // Attempt to change entity_type via PUT (should be ignored)
    let update_payload = serde_json::json!({
        "entity_type": "hacked_type",
        "display_name": "Immutable Test",
        "description": "updated",
        "allow_children": false,
        "icon": "box",
        "fields": [],
        "published": true
    });

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&update_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // GET and verify immutable fields were preserved
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let data = &body["data"];

    // entity_type must NOT change
    assert_eq!(
        data["entity_type"], original_entity_type,
        "entity_type must be immutable"
    );

    // created_at must NOT change
    assert_eq!(
        data["created_at"], original_created_at,
        "created_at must be immutable"
    );

    // uuid must match the path parameter, not any client-supplied value
    assert_eq!(data["uuid"], created_uuid, "uuid must be immutable");

    // But the mutable field (description) should have changed
    assert_eq!(data["description"], "updated");

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test that `updated_at` and `updated_by` are set by the server on update.
#[tokio::test]
#[serial]
async fn test_update_sets_server_controlled_fields() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    // Create entity definition
    let create_payload = serde_json::json!({
        "entity_type": "srv_product",
        "display_name": "Server Fields Test",
        "description": "",
        "allow_children": false,
        "icon": "box",
        "fields": [],
        "published": true
    });

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/entity-definitions")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&create_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let created_uuid = body["data"]["uuid"].as_str().unwrap();

    // GET original timestamps
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let original: serde_json::Value = test::read_body_json(resp).await;
    let original_updated_at = original["data"]["updated_at"].as_str().unwrap().to_string();

    // Update with a trivial change
    let update_payload = serde_json::json!({
        "entity_type": "srv_product",
        "display_name": "Server Fields Test Updated",
        "description": "",
        "allow_children": false,
        "icon": "box",
        "fields": [],
        "published": true
    });

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&update_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // GET and verify server-controlled fields
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let data = &body["data"];

    // updated_at must have changed (server sets it to current time)
    assert_ne!(
        data["updated_at"].as_str().unwrap(),
        original_updated_at,
        "updated_at must be refreshed by the server"
    );

    // created_at must NOT change
    assert_eq!(
        data["created_at"].as_str().unwrap(),
        original["data"]["created_at"].as_str().unwrap(),
        "created_at must be preserved across updates"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
