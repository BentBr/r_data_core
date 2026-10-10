#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix handlers take HttpRequest which is !Send

use time::OffsetDateTime;

use crate::admin::auth::models::{AdminLoginResponse, RefreshTokenResponse};
use crate::admin::auth::rate_limit;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::response::ApiResponse;
use crate::token_service::TokenService;
use r_data_core_core::admin_user::AdminUser;
use r_data_core_core::config::SecurityConfig;
use r_data_core_core::system_log::SystemLogStatus;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_persistence::{RefreshTokenRepository, RefreshTokenRepositoryTrait};

/// Load roles for a user
pub async fn load_user_roles(
    user: &AdminUser,
    data: &ApiStateWrapper,
    repo: &AdminUserRepository,
) -> Vec<r_data_core_core::permissions::role::Role> {
    if user.super_admin {
        // Super admin doesn't need roles - handled in JWT generation
        vec![]
    } else {
        // Load all user's roles
        match data
            .role_service()
            .get_roles_for_user(user.uuid, repo)
            .await
        {
            Ok(r) => {
                log::debug!("Loaded {} roles for user {}", r.len(), user.username);
                r
            }
            Err(e) => {
                log::warn!("Failed to load roles for user: {e}");
                vec![]
            }
        }
    }
}

/// Check if admin user is still using the default password
pub async fn check_admin_default_password(repo: &AdminUserRepository) -> bool {
    // Default password hash from migration
    const DEFAULT_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AyU4SymrYGzpmYfqDSbugg$AhzMvJ1bOxrv2WQ1ks3PRFXGezp966kjJwkoUdJbFY4";

    match repo.find_by_username_or_email("admin").await {
        Ok(Some(admin_user)) => admin_user.password_hash == DEFAULT_PASSWORD_HASH,
        Ok(None) => false,
        Err(e) => {
            log::warn!("Failed to check default admin password: {e:?}");
            false
        }
    }
}

/// Generate tokens and build login response
pub(super) fn build_login_response(
    user: &AdminUser,
    access_token: String,
    refresh_token: String,
    access_expires_at: OffsetDateTime,
    refresh_expires_at: OffsetDateTime,
    using_default_password: bool,
) -> actix_web::HttpResponse {
    ApiResponse::ok(AdminLoginResponse {
        access_token,
        refresh_token,
        user_uuid: user.uuid.to_string(),
        username: user.username.clone(),
        access_expires_at,
        refresh_expires_at,
        using_default_password,
    })
}

/// Build refresh token response
pub(super) fn build_refresh_response(
    access_token: String,
    refresh_token: String,
    access_expires_at: OffsetDateTime,
    refresh_expires_at: OffsetDateTime,
) -> actix_web::HttpResponse {
    ApiResponse::ok(RefreshTokenResponse {
        access_token,
        refresh_token,
        access_expires_at,
        refresh_expires_at,
    })
}

/// Record a failed password attempt: mutate lockout state, increment IP counter, log, return 401.
pub async fn handle_password_failure(
    mut user: AdminUser,
    repo: &AdminUserRepository,
    data: &ApiStateWrapper,
    rl_key: &str,
    username: &str,
) -> actix_web::HttpResponse {
    let security = SecurityConfig::global();
    user.record_login_failure(security.max_failed_attempts, security.lockout_duration_secs);
    if let Err(e) = repo
        .update_lockout_state(
            &user.uuid,
            &user.status,
            user.failed_login_attempts,
            user.locked_until,
        )
        .await
    {
        log::error!("Failed to persist lockout state: {e:?}");
    }
    rate_limit::record_failure(data.cache_manager(), rl_key).await;
    log::debug!("Password verification failed for user: {username}");
    if let Some(log_svc) = data.system_log_service() {
        log_svc
            .log_auth_event(
                None,
                None,
                "Login failed: invalid credentials",
                Some(serde_json::json!({"action": "login", "reason": "invalid_credentials"})),
                SystemLogStatus::Failed,
            )
            .await;
    }
    ApiResponse::unauthorized("Invalid credentials")
}

/// The session artefacts a successful sign-in produces.
pub struct IssuedSession {
    pub tokens: crate::token_service::TokenPair,
    pub using_default_password: bool,
}

/// Reset lockout state, persist last-login, issue tokens, store refresh token.
///
/// Every way of signing in ends here — password login and the single-sign-on
/// callback both — so an SSO session is the *same* session a password login
/// produces. That is what lets refresh, logout and revoke-all work with no
/// SSO-specific branches anywhere in the session code.
///
/// Returns `None` when something internal failed; the reason is logged, and
/// callers should answer with a generic failure rather than say more.
pub async fn issue_session(
    user: &mut AdminUser,
    repo: &AdminUserRepository,
    data: &ApiStateWrapper,
) -> Option<IssuedSession> {
    user.record_login_success();
    if let Err(e) = repo
        .update_lockout_state(&user.uuid, &user.status, user.failed_login_attempts, None)
        .await
    {
        log::error!("Failed to reset lockout state: {e:?}");
    }
    if let Err(e) = repo.update_last_login(&user.uuid).await {
        log::error!("Failed to update last login: {e:?}");
    }

    let roles = load_user_roles(user, data, repo).await;
    let token_service = TokenService::new(data.api_config());
    let token_pair = match token_service.generate_token_pair(user, &roles) {
        Ok(pair) => pair,
        Err(e) => {
            log::error!("Failed to generate tokens: {e:?}");
            return None;
        }
    };

    let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());
    let device_info = Some(serde_json::json!({
        "user_agent": "login",
        "login_time": OffsetDateTime::now_utc()
    }));
    if let Err(e) = refresh_repo
        .create(
            user.uuid,
            token_pair.refresh_token_hash.clone(),
            token_pair.refresh_expires_at,
            device_info,
        )
        .await
    {
        log::error!("Failed to store refresh token: {e:?}");
        return None;
    }

    let using_default_password = if data.api_config().check_default_admin_password {
        check_admin_default_password(repo).await
    } else {
        false
    };

    if let Some(log_svc) = data.system_log_service() {
        let user_uuid = user.uuid;
        log_svc
            .log_auth_event(
                Some(user_uuid),
                Some(user_uuid),
                "User logged in",
                Some(serde_json::json!({"action": "login"})),
                SystemLogStatus::Success,
            )
            .await;
    }

    Some(IssuedSession {
        tokens: token_pair,
        using_default_password,
    })
}

/// Issue a session and answer with the standard login response.
pub async fn complete_login(
    mut user: AdminUser,
    repo: AdminUserRepository,
    data: &ApiStateWrapper,
) -> actix_web::HttpResponse {
    let Some(session) = issue_session(&mut user, &repo, data).await else {
        return ApiResponse::internal_error("Authentication failed");
    };

    build_login_response(
        &user,
        session.tokens.access_token,
        session.tokens.refresh_token,
        session.tokens.access_expires_at,
        session.tokens.refresh_expires_at,
        session.using_default_password,
    )
}
