#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Fakes shared by the OIDC test modules.
//!
//! Fakes rather than a database, so the ordering guarantees these tests exist
//! to protect can be checked directly: that a rejected identity leaves no user
//! row, and that a deactivated account is refused however valid its token.
//!
//! One copy, used by both `oidc_provisioning` and `oidc_runtime`. A second set
//! would drift, and the two modules assert about the same invariants.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use r_data_core_core::admin_user::{AdminUser, UserStatus};
use r_data_core_core::domain::AbstractRDataEntity;
use r_data_core_core::error::Result;
use r_data_core_core::oidc::keys::OidcClaims;
use r_data_core_core::oidc::OidcConfig;
use r_data_core_core::permissions::role::Role;
use r_data_core_persistence::admin_user_repository_trait::{
    AdminUserRepositoryTrait, CreateAdminUserParams,
};
use r_data_core_persistence::{IdentityRepositoryTrait, RoleRepositoryTrait};

use crate::oidc_provisioning::OidcProvisioningService;

// ── fakes ───────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct FakeIdentities {
    pub links: Mutex<HashMap<(String, String), Uuid>>,
    pub local_by_email: Mutex<HashMap<String, Uuid>>,
    pub marked_sso: Mutex<Vec<Uuid>>,
}

#[async_trait]
impl IdentityRepositoryTrait for FakeIdentities {
    async fn find_user_by_identity(&self, provider: &str, subject: &str) -> Result<Option<Uuid>> {
        Ok(self
            .links
            .lock()
            .expect("lock")
            .get(&(provider.to_string(), subject.to_string()))
            .copied())
    }

    async fn link_identity(
        &self,
        provider: &str,
        subject: &str,
        admin_user_uuid: Uuid,
    ) -> Result<Uuid> {
        let mut links = self.links.lock().expect("lock");
        // First link wins, matching the real ON CONFLICT DO NOTHING.
        Ok(*links
            .entry((provider.to_string(), subject.to_string()))
            .or_insert(admin_user_uuid))
    }

    async fn touch_last_login(&self, _provider: &str, _subject: &str) -> Result<()> {
        Ok(())
    }

    async fn mark_sso_provisioned(&self, admin_user_uuid: Uuid) -> Result<()> {
        self.marked_sso.lock().expect("lock").push(admin_user_uuid);
        Ok(())
    }

    async fn find_identities_for_user(
        &self,
        admin_user_uuid: Uuid,
    ) -> Result<Vec<(String, String)>> {
        Ok(self
            .links
            .lock()
            .expect("lock")
            .iter()
            .filter(|(_, uuid)| **uuid == admin_user_uuid)
            .map(|((provider, subject), _)| (provider.clone(), subject.clone()))
            .collect())
    }

    async fn find_local_user_by_email(&self, email: &str) -> Result<Option<Uuid>> {
        Ok(self
            .local_by_email
            .lock()
            .expect("lock")
            .get(email)
            .copied())
    }
}

#[derive(Default)]
pub struct FakeUsers {
    pub users: Mutex<Vec<AdminUser>>,
    pub created: Mutex<Vec<String>>,
    /// Addresses `create_admin_user` must refuse with a unique violation, as
    /// the real table does.
    pub taken_emails: Mutex<Vec<String>>,
}

impl FakeUsers {
    pub fn with(users: Vec<AdminUser>) -> Self {
        Self {
            users: Mutex::new(users),
            created: Mutex::new(Vec::new()),
            taken_emails: Mutex::new(Vec::new()),
        }
    }
    /// Deactivate an account, as an operator would.
    pub fn deactivate(&self, uuid: Uuid) {
        for user in self.users.lock().expect("lock").iter_mut() {
            if user.uuid == uuid {
                user.is_active = false;
            }
        }
    }

    pub fn created_count(&self) -> usize {
        self.created.lock().expect("lock").len()
    }
}

pub fn user(uuid: Uuid, username: &str, active: bool) -> AdminUser {
    AdminUser {
        base: AbstractRDataEntity::new("/admin/users".to_string()),
        username: username.to_string(),
        email: format!("{username}@example.com"),
        password_hash: "unusable".to_string(),
        full_name: username.to_string(),
        status: UserStatus::Active,
        last_login: None,
        failed_login_attempts: 0,
        locked_until: None,
        super_admin: false,
        is_sso_provisioned: false,
        uuid,
        first_name: None,
        last_name: None,
        is_active: active,
        is_admin: false,
        created_at: time::OffsetDateTime::now_utc(),
        updated_at: time::OffsetDateTime::now_utc(),
    }
}

pub fn super_admin(uuid: Uuid, username: &str) -> AdminUser {
    let mut user = user(uuid, username, true);
    user.super_admin = true;
    user
}

#[async_trait]
impl AdminUserRepositoryTrait for FakeUsers {
    async fn find_by_username_or_email(&self, needle: &str) -> Result<Option<AdminUser>> {
        Ok(self
            .users
            .lock()
            .expect("lock")
            .iter()
            .find(|u| u.username == needle || u.email == needle)
            .cloned())
    }

    async fn find_by_uuid(&self, uuid: &Uuid) -> Result<Option<AdminUser>> {
        Ok(self
            .users
            .lock()
            .expect("lock")
            .iter()
            .find(|u| u.uuid == *uuid)
            .cloned())
    }

    async fn update_last_login(&self, _uuid: &Uuid) -> Result<()> {
        Ok(())
    }

    async fn create_admin_user<'a>(&self, params: &CreateAdminUserParams<'a>) -> Result<Uuid> {
        if self
            .taken_emails
            .lock()
            .expect("lock")
            .iter()
            .any(|taken| taken.eq_ignore_ascii_case(params.email))
        {
            // What sqlx reports for a unique violation on admin_users.email.
            return Err(r_data_core_core::error::Error::Database(
                sqlx::Error::Database(Box::new(FakeUniqueViolation)),
            ));
        }
        let uuid = Uuid::now_v7();
        self.created
            .lock()
            .expect("lock")
            .push(params.username.to_string());
        let mut account = user(uuid, params.username, params.is_active);
        account.email = params.email.to_string();
        self.users.lock().expect("lock").push(account);
        Ok(uuid)
    }

    async fn update_admin_user(&self, _user: &AdminUser) -> Result<()> {
        Ok(())
    }
    async fn delete_admin_user(&self, _uuid: &Uuid) -> Result<()> {
        Ok(())
    }
    async fn update_lockout_state(
        &self,
        _uuid: &Uuid,
        _status: &UserStatus,
        _failed_login_attempts: i32,
        _locked_until: Option<time::OffsetDateTime>,
    ) -> Result<()> {
        Ok(())
    }
    async fn list_admin_users(
        &self,
        _limit: i64,
        _offset: i64,
        _sort_by: Option<String>,
        _sort_order: Option<String>,
    ) -> Result<Vec<AdminUser>> {
        Ok(self.users.lock().expect("lock").clone())
    }
}

pub struct FakeRoles {
    pub roles: Vec<Role>,
    /// Stored `user_roles`, as `(user, role)` pairs.
    pub user_roles: Vec<(Uuid, Role)>,
}

#[async_trait]
impl RoleRepositoryTrait for FakeRoles {
    async fn get_by_uuid(&self, _uuid: Uuid) -> Result<Option<Role>> {
        Ok(None)
    }
    async fn get_by_name(&self, name: &str) -> Result<Option<Role>> {
        Ok(self.roles.iter().find(|r| r.name == name).cloned())
    }
    async fn create(&self, _role: &Role, _created_by: Uuid) -> Result<Uuid> {
        Ok(Uuid::now_v7())
    }
    async fn update(&self, _role: &Role, _updated_by: Uuid) -> Result<()> {
        Ok(())
    }
    async fn delete(&self, _uuid: Uuid) -> Result<()> {
        Ok(())
    }
    async fn list_all(
        &self,
        _limit: i64,
        _offset: i64,
        _sort_by: Option<String>,
        _sort_order: Option<String>,
    ) -> Result<Vec<Role>> {
        Ok(self.roles.clone())
    }
    async fn count_all(&self) -> Result<i64> {
        Ok(i64::try_from(self.roles.len()).unwrap_or(0))
    }
    async fn list_for_user(&self, user_uuid: Uuid) -> Result<Vec<Role>> {
        Ok(self
            .user_roles
            .iter()
            .filter(|(user, _)| *user == user_uuid)
            .map(|(_, role)| role.clone())
            .collect())
    }
}

// ── fixtures ────────────────────────────────────────────────────────────────

pub const ISSUER: &str = "https://auth.example.com";

pub fn claims(subject: &str, groups: &[&str], email_verified: bool) -> OidcClaims {
    OidcClaims {
        issuer: ISSUER.to_string(),
        subject: subject.to_string(),
        email: Some(format!("{subject}@example.com")),
        email_verified,
        name: Some("Ada Lovelace".to_string()),
        groups: groups.iter().map(|g| (*g).to_string()).collect(),
        raw: HashMap::new(),
    }
}

pub fn config(default_role: Option<&str>, link_by_email: bool) -> OidcConfig {
    let mut pairs = vec![
        ("RDC_OIDC_ISSUER".to_string(), ISSUER.to_string()),
        ("RDC_OIDC_AUDIENCE".to_string(), "r-data-core".to_string()),
        (
            "RDC_OIDC_ROLE_MAP".to_string(),
            "rdc-ops:editor".to_string(),
        ),
    ];
    if let Some(role) = default_role {
        pairs.push(("RDC_OIDC_DEFAULT_ROLE".to_string(), role.to_string()));
    }
    if link_by_email {
        pairs.push(("RDC_OIDC_LINK_BY_EMAIL".to_string(), "true".to_string()));
    }
    OidcConfig::from_map(&pairs.into_iter().collect())
        .expect("valid")
        .expect("enabled")
}

pub fn service(identities: Arc<FakeIdentities>, users: Arc<FakeUsers>) -> OidcProvisioningService {
    service_with_user_roles(identities, users, Vec::new())
}

/// [`service`], with roles already stored for some users.
pub fn service_with_user_roles(
    identities: Arc<FakeIdentities>,
    users: Arc<FakeUsers>,
    user_roles: Vec<(Uuid, Role)>,
) -> OidcProvisioningService {
    OidcProvisioningService::new(
        identities,
        users,
        Arc::new(FakeRoles {
            roles: vec![
                Role::new("editor".to_string()),
                Role::new("viewer".to_string()),
            ],
            user_roles,
        }),
    )
}

/// A `23505` as the driver would surface it.
#[derive(Debug)]
pub struct FakeUniqueViolation;

impl std::fmt::Display for FakeUniqueViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("duplicate key value violates unique constraint")
    }
}

impl std::error::Error for FakeUniqueViolation {}

impl sqlx::error::DatabaseError for FakeUniqueViolation {
    fn message(&self) -> &'static str {
        "duplicate key value violates unique constraint"
    }
    fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
        Some("23505".into())
    }
    fn kind(&self) -> sqlx::error::ErrorKind {
        sqlx::error::ErrorKind::UniqueViolation
    }
    fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }
    fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        self
    }
    fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self
    }
}
