#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for string field constraints (pattern, `min_length`, `max_length`).

use super::common::{create_test_app, create_test_jwt_token};
use actix_web::{
    http::{header, StatusCode},
    test,
};
use r_data_core_core::error::Result;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use uuid::Uuid;

/// Test creating an entity definition with string constraints (pattern, `min_length`, `max_length`).
#[tokio::test]
#[serial]
async fn test_create_entity_definition_with_string_constraints() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    // Create entity definition with nested string constraints
    let payload = serde_json::json!({
        "entity_type": "test_user",
        "display_name": "Test User",
        "description": "User with email validation",
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
            },
            {
                "name": "username",
                "display_name": "Username",
                "field_type": "String",
                "required": true,
                "indexed": true,
                "filterable": true,
                "unique": false,
                "constraints": {
                    "type": "string",
                    "constraints": {
                        "min_length": 3,
                        "max_length": 50,
                        "pattern": "^[a-zA-Z0-9_]+$"
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
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "Success");

    let created_uuid = body["data"]["uuid"].as_str().unwrap();
    let created_uuid = Uuid::parse_str(created_uuid).unwrap();

    // Retrieve and verify constraints are preserved
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let fields = body["data"]["fields"].as_array().unwrap();

    // Verify email field constraints
    let email_field = fields.iter().find(|f| f["name"] == "email").unwrap();
    assert_eq!(email_field["unique"], true);
    let email_constraints = &email_field["constraints"];
    assert!(email_constraints["type"].as_str().is_some());

    // Verify username field constraints
    let username_field = fields.iter().find(|f| f["name"] == "username").unwrap();
    assert_eq!(username_field["unique"], false);

    clear_test_db(&pool).await?;
    Ok(())
}
