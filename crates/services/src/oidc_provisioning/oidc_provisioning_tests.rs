#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Fakes rather than a database, so the ordering guarantees these tests exist
//! to protect can be checked directly: that a rejected identity leaves no user
//! row, and that a deactivated account is refused however valid its token.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use r_data_core_core::admin_user::UserStatus;
use r_data_core_core::domain::AbstractRDataEntity;

use super::*;

// ── fakes ───────────────────────────────────────────────────────────────────

#[derive(Default)]
struct FakeIdentities {
    links: Mutex<HashMap<(String, String), Uuid>>,
    local_by_email: Mutex<HashMap<String, Uuid>>,
    marked_sso: Mutex<Vec<Uuid>>,
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
struct FakeUsers {
    users: Mutex<Vec<AdminUser>>,
    created: Mutex<Vec<String>>,
}

impl FakeUsers {
    fn with(users: Vec<AdminUser>) -> Self {
        Self {
            users: Mutex::new(users),
            created: Mutex::new(Vec::new()),
        }
    }
    fn created_count(&self) -> usize {
        self.created.lock().expect("lock").len()
    }
}

fn user(uuid: Uuid, username: &str, active: bool) -> AdminUser {
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
        let uuid = Uuid::now_v7();
        self.created
            .lock()
            .expect("lock")
            .push(params.username.to_string());
        self.users
            .lock()
            .expect("lock")
            .push(user(uuid, params.username, params.is_active));
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

struct FakeRoles {
    roles: Vec<Role>,
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
}

// ── fixtures ────────────────────────────────────────────────────────────────

const ISSUER: &str = "https://auth.example.com";

fn claims(subject: &str, groups: &[&str], email_verified: bool) -> OidcClaims {
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

fn config(default_role: Option<&str>, link_by_email: bool) -> OidcConfig {
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

fn service(identities: Arc<FakeIdentities>, users: Arc<FakeUsers>) -> OidcProvisioningService {
    OidcProvisioningService::new(
        identities,
        users,
        Arc::new(FakeRoles {
            roles: vec![
                Role::new("editor".to_string()),
                Role::new("viewer".to_string()),
            ],
        }),
    )
}

// ── tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_first_sign_in_provisions_an_account() {
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::default());

    let (user, roles) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(
            &claims("new-subject", &["rdc-ops"], true),
            &config(None, false),
        )
        .await
        .expect("should provision");

    assert_eq!(users.created_count(), 1);
    assert_eq!(roles[0].name, "editor");
    assert_eq!(user.username, "new-subject");
    assert!(
        identities
            .marked_sso
            .lock()
            .expect("lock")
            .contains(&user.uuid),
        "a provisioned account must be flagged, or the password paths stay open"
    );
}

#[tokio::test]
async fn a_known_identity_resolves_without_creating_anything() {
    let existing = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .links
        .lock()
        .expect("lock")
        .insert((ISSUER.to_string(), "known".to_string()), existing);
    let users = Arc::new(FakeUsers::with(vec![user(existing, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("known", &["rdc-ops"], true), &config(None, false))
        .await
        .expect("should resolve");

    assert_eq!(resolved.uuid, existing);
    assert_eq!(users.created_count(), 0);
}

#[tokio::test]
async fn a_rejected_identity_leaves_no_account_behind() {
    // Roles are resolved before anything is created, precisely so this holds.
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::default());

    let result = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(
            &claims("unmapped", &["some-other-team"], true),
            &config(None, false),
        )
        .await;

    assert!(result.is_err(), "an unmapped identity must be refused");
    assert_eq!(
        users.created_count(),
        0,
        "a refused sign-in must not leave a user row"
    );
    assert!(identities.links.lock().expect("lock").is_empty());
}

#[tokio::test]
async fn a_deactivated_account_is_refused_despite_a_valid_token() {
    // The provider says who someone is; RDataCore says whether they may act.
    let existing = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .links
        .lock()
        .expect("lock")
        .insert((ISSUER.to_string(), "disabled".to_string()), existing);
    let users = Arc::new(FakeUsers::with(vec![user(existing, "ada", false)]));

    let err = service(identities, users)
        .resolve_or_provision(
            &claims("disabled", &["rdc-ops"], true),
            &config(None, false),
        )
        .await
        .expect_err("a deactivated account must not sign in");
    assert!(format!("{err}").contains("deactivated"), "{err}");
}

#[tokio::test]
async fn a_locked_account_is_refused() {
    let existing = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .links
        .lock()
        .expect("lock")
        .insert((ISSUER.to_string(), "locked".to_string()), existing);
    let mut locked = user(existing, "ada", true);
    locked.locked_until = Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1));
    let users = Arc::new(FakeUsers::with(vec![locked]));

    let err = service(identities, users)
        .resolve_or_provision(&claims("locked", &["rdc-ops"], true), &config(None, false))
        .await
        .expect_err("a locked account must not sign in");
    assert!(format!("{err}").contains("locked"), "{err}");
}

#[tokio::test]
async fn email_linking_is_refused_when_the_address_is_unverified() {
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ada@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], false), &config(None, true))
        .await
        .expect("should provision a new account instead");

    assert_ne!(
        resolved.uuid, local,
        "an unverified address must not adopt an existing account"
    );
    assert_eq!(users.created_count(), 1);
}

#[tokio::test]
async fn email_linking_is_refused_when_the_operator_has_not_enabled_it() {
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ada@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, false))
        .await
        .expect("should provision");

    assert_ne!(resolved.uuid, local, "linking is off by default");
}

#[tokio::test]
async fn email_linking_adopts_the_account_when_enabled_and_verified() {
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ada@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, true))
        .await
        .expect("should adopt");

    assert_eq!(resolved.uuid, local);
    assert_eq!(users.created_count(), 0, "no new account for an adoption");
    assert!(
        identities.marked_sso.lock().expect("lock").is_empty(),
        "an adopted local account keeps its password; only created ones are flagged"
    );
}

#[tokio::test]
async fn a_username_collision_does_not_fail_the_sign_in() {
    // Two people at different providers can share an email local-part.
    let taken = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::with(vec![user(taken, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, false))
        .await
        .expect("a collision must not block sign-in");

    assert_ne!(resolved.username, "ada");
    assert!(
        resolved.username.starts_with("ada-"),
        "got {}",
        resolved.username
    );
}

#[tokio::test]
async fn the_default_role_applies_to_an_unmapped_identity() {
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::default());

    let (_, roles) = service(identities, users)
        .resolve_or_provision(
            &claims("unmapped", &["nothing-matches"], true),
            &config(Some("viewer"), false),
        )
        .await
        .expect("the default should apply");
    assert_eq!(roles[0].name, "viewer");
}

#[test]
fn a_display_name_is_split_into_first_and_last() {
    assert_eq!(
        split_name(Some("Ada Lovelace")),
        ("Ada".into(), "Lovelace".into())
    );
    assert_eq!(split_name(Some("Ada")), ("Ada".into(), String::new()));
    assert_eq!(
        split_name(Some("Ada King Lovelace")),
        ("Ada".into(), "King Lovelace".into())
    );
    assert_eq!(split_name(None), (String::new(), String::new()));
    assert_eq!(split_name(Some("   ")), (String::new(), String::new()));
}

#[test]
fn the_sentinel_password_is_long_and_never_the_same_twice() {
    let a = sentinel_password();
    let b = sentinel_password();
    assert_eq!(a.chars().count(), SENTINEL_PASSWORD_BYTES);
    assert_ne!(
        a, b,
        "a fixed sentinel would be shared across installations"
    );
}
