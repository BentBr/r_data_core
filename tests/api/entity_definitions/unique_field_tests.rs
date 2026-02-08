#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for unique field constraints.

use super::common::{create_test_app, create_test_jwt_token};
use actix_web::{
    http::{header, StatusCode},
    test,
};
use r_data_core_core::error::Result;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;

/// Test that unique field generates proper constraint indicator in response.
#[tokio::test]
#[serial]
async fn test_unique_field_indicator_in_response() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    let payload = serde_json::json!({
        "entity_type": "test_account",
        "display_name": "Test Account",
        "description": "Account with unique email",
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
                "unique": true
            },
            {
                "name": "name",
                "display_name": "Name",
                "field_type": "String",
                "required": true,
                "indexed": false,
                "filterable": false,
                "unique": false
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

    // Get the created UUID from the response
    let created_uuid = body["data"]["uuid"]
        .as_str()
        .expect("UUID should be in create response");

    // Fetch the created entity definition to verify fields
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let fields = body["data"]["fields"]
        .as_array()
        .expect("fields should be an array in GET response");

    // Verify unique field is true for email
    let email_field = fields
        .iter()
        .find(|f| f["name"] == "email")
        .expect("email field should exist in response");
    assert_eq!(
        email_field["unique"], true,
        "Email field should have unique=true"
    );

    // Verify unique field is false for name
    let name_field = fields
        .iter()
        .find(|f| f["name"] == "name")
        .expect("name field should exist in response");
    assert_eq!(
        name_field["unique"], false,
        "Name field should have unique=false"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
