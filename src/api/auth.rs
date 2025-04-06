use actix_web::{web, HttpResponse, Responder, post};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use chrono::{Utc, Duration};
use serde::{Serialize, Deserialize};
use log::error;

use crate::entity::AdminUser;
use crate::entity::admin_user::UserRole;
use crate::error::{Error, Result};
use crate::api::ApiState;
use utoipa::ToSchema;
use crate::db::repository::PgPoolExtension;

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    
    /// Expiration time (as UTC timestamp)
    pub exp: i64,
    
    /// Issued at (as UTC timestamp)
    pub iat: i64,
    
    /// User role
    pub role: String,
    
    /// User permissions (optional)
    pub permissions: Option<Vec<String>>,
}

/// Login request body
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Username or email
    pub username: String,
    
    /// Password
    pub password: String,
}

/// Login response body
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    /// JWT token
    pub token: String,
    
    /// User ID
    pub user_id: i64,
    
    /// User UUID
    pub uuid: String,
    
    /// Username
    pub username: String,
    
    /// User role
    pub role: String,
    
    /// Token expiration (timestamp)
    pub expires_at: i64,
}

/// Claims for authentication
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthUserClaims {
    pub sub: i64,             // User ID
    pub name: String,         // Username
    pub email: String,        // Email
    pub is_admin: bool,       // Admin flag
    pub exp: usize,           // Expiration timestamp
    pub iat: usize,           // Issued at timestamp
}

/// Login endpoint
#[post("/login")]
pub async fn login(
    data: web::Data<ApiState>,
    login_req: web::Json<LoginRequest>,
) -> impl Responder {
    // Get database connection
    let db_pool = &data.db_pool;
    
    // Find user by username or email
    let user_result = sqlx::query_as::<_, AdminUser>(
        "SELECT * FROM admin_users WHERE username = $1 OR email = $1",
    )
    .bind(&login_req.username)
    .fetch_optional(db_pool)
    .await;
    
    let user = match user_result {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid username or password"
            }));
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database error"
            }));
        }
    };
    
    // Verify password
    if !user.verify_password(&login_req.password) {
        // TODO: In a real implementation, we should update the user's
        // failed login attempts here
        
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid username or password"
        }));
    }
    
    // Check if user is active
    if !user.can_login() {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Account is inactive or locked"
        }));
    }
    
    // Generate JWT token
    let token = match generate_jwt(&user, &data.jwt_secret, 86400) {
        Ok(token) => token,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Could not generate authentication token"
            }));
        }
    };
    
    // Calculate expiration time
    let expires_at = Utc::now()
        .checked_add_signed(Duration::seconds(86400))
        .unwrap_or(Utc::now())
        .timestamp();
        
    // Build response
    let response = LoginResponse {
        token,
        user_id: user.base.id.unwrap(),
        uuid: user.base.uuid.to_string(),
        username: user.username,
        role: format!("{:?}", user.role),
        expires_at,
    };
    
    // TODO: In a real implementation, we should update the user's
    // last login time and reset failed login attempts here
    
    HttpResponse::Ok().json(response)
}

/// User registration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// Username
    pub username: String,
    
    /// Email
    pub email: String,
    
    /// Password
    pub password: String,
    
    /// Full name
    pub full_name: String,
}

/// Register a new user endpoint
#[post("/register")]
pub async fn register(
    data: web::Data<ApiState>,
    register_req: web::Json<RegisterRequest>,
) -> impl Responder {
    // Get database connection
    let db_pool = &data.db_pool;
    
    // Check if username or email already exists
    let existing_user = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM admin_users WHERE username = $1 OR email = $2",
    )
    .bind(&register_req.username)
    .bind(&register_req.email)
    .fetch_optional(db_pool)
    .await;
    
    if let Ok(Some(_)) = existing_user {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Username or email already exists"
        }));
    }
    
    // Create new user
    let mut user = AdminUser::new(
        register_req.username.clone(),
        register_req.email.clone(),
        register_req.full_name.clone(),
    );
    
    // Set password
    if let Err(e) = user.set_password(&register_req.password) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Could not hash password: {}", e)
        }));
    }
    
    // Save user to database
    let result = db_pool.repository_with_table::<AdminUser>("admin_users").create(&user).await;
    
    match result {
        Ok(id) => {
            user.base.id = Some(id);
            
            // Return success response
            HttpResponse::Created().json(serde_json::json!({
                "id": id,
                "uuid": user.base.uuid.to_string(),
                "username": user.username,
                "message": "User registered successfully"
            }))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to register user: {}", e)
            }))
        }
    }
}

/// Register authentication routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/auth")
            .service(login)
            .service(register)
    );
}

/// Generate a JWT token for a user
pub fn generate_jwt(user: &AdminUser, secret: &str, expiration_seconds: u64) -> Result<String> {
    let user_id = user.base.id.ok_or_else(|| Error::Auth("User has no ID".to_string()))?;
    
    // Create expiration time
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(expiration_seconds as i64))
        .ok_or_else(|| Error::Auth("Could not create token expiration".to_string()))?;
        
    // Create claims
    let claims = AuthUserClaims {
        sub: user_id,
        name: user.username.clone(),
        email: user.email.clone(),
        is_admin: user.role == UserRole::Admin,
        exp: expiration.timestamp() as usize,
        iat: Utc::now().timestamp() as usize,
    };
    
    // Generate the token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| Error::Auth(format!("Token generation error: {}", e)))?;
    
    Ok(token)
}

/// Verify and decode a JWT token
pub fn verify_jwt(token: &str, secret: &str) -> Result<AuthUserClaims> {
    // Decode and validate the token
    let token_data = decode::<AuthUserClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| Error::Auth(format!("Token validation error: {}", e)))?;
    
    Ok(token_data.claims)
}

// Add AdminOnly extractor if needed
pub struct AdminOnly(pub AuthUserClaims); 