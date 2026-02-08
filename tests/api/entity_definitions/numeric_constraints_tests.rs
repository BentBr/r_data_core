#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for numeric field constraints (min, max, `positive_only`).

use super::common::{create_test_app, create_test_jwt_token};
use actix_web::{
    http::{header, StatusCode},
    test,
};
use r_data_core_core::error::Result;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use uuid::Uuid;

/// Test creating an entity definition with numeric constraints (min, max, `positive_only`).
#[tokio::test]
#[serial]
async fn test_create_entity_definition_with_numeric_constraints() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    let payload = serde_json::json!({
        "entity_type": "test_product",
        "display_name": "Test Product",
        "description": "Product with numeric constraints",
        "group_name": "test",
        "allow_children": false,
        "icon": "package",
        "fields": [
            {
                "name": "price",
                "display_name": "Price",
                "field_type": "Float",
                "required": true,
                "indexed": true,
                "filterable": true,
                "constraints": {
                    "type": "float",
                    "constraints": {
                        "min": 0,
                        "max": 999_999.99,
                        "positive_only": true
                    }
                }
            },
            {
                "name": "quantity",
                "display_name": "Quantity",
                "field_type": "Integer",
                "required": true,
                "indexed": false,
                "filterable": true,
                "constraints": {
                    "type": "integer",
                    "constraints": {
                        "min": 0,
                        "max": 10000
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

    // Retrieve and verify constraints
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let fields = body["data"]["fields"].as_array().unwrap();

    // Verify price field constraints
    let price_field = fields.iter().find(|f| f["name"] == "price").unwrap();
    let price_constraints = &price_field["constraints"];
    assert!(price_constraints.is_object());

    // Verify quantity field constraints
    let quantity_field = fields.iter().find(|f| f["name"] == "quantity").unwrap();
    let quantity_constraints = &quantity_field["constraints"];
    assert!(quantity_constraints.is_object());

    clear_test_db(&pool).await?;
    Ok(())
}
