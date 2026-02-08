#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for updating entity definition field constraints.

use super::common::{create_test_app, create_test_jwt_token};
use actix_web::{
    http::{header, StatusCode},
    test,
};
use r_data_core_core::error::Result;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;

/// Test updating an entity definition's field constraints.
#[tokio::test]
#[serial]
async fn test_update_entity_definition_field_constraints() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    // Create initial entity definition without constraints
    let create_payload = serde_json::json!({
        "entity_type": "test_customer",
        "display_name": "Test Customer",
        "description": "Customer entity",
        "group_name": "test",
        "allow_children": false,
        "icon": "user",
        "fields": [
            {
                "name": "email",
                "display_name": "Email",
                "field_type": "String",
                "required": true,
                "indexed": true,
                "filterable": true,
                "unique": false
            }
        ],
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

    // Update with constraints
    let update_payload = serde_json::json!({
        "entity_type": "test_customer",
        "display_name": "Test Customer",
        "description": "Customer entity with validation",
        "group_name": "test",
        "allow_children": false,
        "icon": "user",
        "fields": [
            {
                "name": "email",
                "display_name": "Email",
                "field_type": "String",
                "required": true,
                "indexed": true,
                "filterable": true,
                "unique": true,
                "constraints": {
                    "type": "string",
                    "constraints": {
                        "pattern": "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$",
                        "min_length": 5,
                        "max_length": 255
                    }
                }
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

    // Verify update
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let fields = body["data"]["fields"].as_array().unwrap();
    let email_field = fields.iter().find(|f| f["name"] == "email").unwrap();

    assert_eq!(email_field["unique"], true);
    let constraints = &email_field["constraints"];
    assert!(constraints.is_object());

    clear_test_db(&pool).await?;
    Ok(())
}
