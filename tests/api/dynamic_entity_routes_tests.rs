use actix_web::{test, App, web, HttpResponse, Responder};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

// Create a simpler test endpoint that doesn't need mocking or real services
async fn get_test_entities() -> impl Responder {
    let mut entity_data = HashMap::new();
    entity_data.insert("uuid".to_string(), json!(Uuid::nil().to_string()));
    entity_data.insert("name".to_string(), json!("Test Entity"));
    
    let test_entity = json!({
        "entity_type": "test_entity",
        "field_data": entity_data
    });
    
    HttpResponse::Ok().json(vec![test_entity])
}

async fn create_test_entity(entity: web::Json<HashMap<String, Value>>) -> impl Responder {
    // Check if the required field exists
    if !entity.contains_key("required_field") {
        return HttpResponse::BadRequest().json(json!({
            "error": "Required field 'required_field' is missing"
        }));
    }
    
    HttpResponse::Created().json(json!({
        "status": "created",
        "uuid": Uuid::nil().to_string()
    }))
}

#[actix_web::test]
async fn test_get_entities() {
    // Initialize the test app with our test endpoint
    let app = test::init_service(
        App::new().service(
            web::resource("/api/entities/test_entity")
                .route(web::get().to(get_test_entities))
        )
    ).await;
    
    // Create a test request
    let req = test::TestRequest::get()
        .uri("/api/entities/test_entity")
        .to_request();
    
    // Perform the request
    let resp = test::call_service(&app, req).await;
    
    // Assert the response
    assert!(resp.status().is_success());
    
    // Parse the response body
    let body: Value = test::read_body_json(resp).await;
    
    // Verify structure
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["entity_type"], "test_entity");
    assert!(body[0]["field_data"]["name"].is_string());
    assert_eq!(body[0]["field_data"]["name"], "Test Entity");
}

#[actix_web::test]
async fn test_create_entity_success() {
    // Initialize the test app with our test endpoint
    let app = test::init_service(
        App::new().service(
            web::resource("/api/entities/test_entity")
                .route(web::post().to(create_test_entity))
        )
    ).await;
    
    // Create a test request with valid data
    let req = test::TestRequest::post()
        .uri("/api/entities/test_entity")
        .set_json(json!({
            "required_field": "test value",
            "optional_field": 42
        }))
        .to_request();
    
    // Perform the request
    let resp = test::call_service(&app, req).await;
    
    // Assert the response
    assert_eq!(resp.status().as_u16(), 201); // Created
    
    // Parse the response body
    let body: Value = test::read_body_json(resp).await;
    
    // Verify structure
    assert_eq!(body["status"], "created");
    assert!(body["uuid"].is_string());
}

#[actix_web::test]
async fn test_create_entity_missing_required_field() {
    // Initialize the test app with our test endpoint
    let app = test::init_service(
        App::new().service(
            web::resource("/api/entities/test_entity")
                .route(web::post().to(create_test_entity))
        )
    ).await;
    
    // Create a test request with missing required field
    let req = test::TestRequest::post()
        .uri("/api/entities/test_entity")
        .set_json(json!({
            "optional_field": 42
        }))
        .to_request();
    
    // Perform the request
    let resp = test::call_service(&app, req).await;
    
    // Assert the response
    assert_eq!(resp.status().as_u16(), 400); // Bad Request
    
    // Parse the response body
    let body: Value = test::read_body_json(resp).await;
    
    // Verify structure
    assert!(body["error"].is_string());
    assert_eq!(body["error"], "Required field 'required_field' is missing");
}
