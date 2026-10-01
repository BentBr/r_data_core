#![allow(clippy::expect_used, clippy::unwrap_used)]

//! That an account which signs in through single sign-on has no password
//! path back in.
//!
//! Both guards matter and neither makes the other redundant. Without the one
//! in `request_reset`, forgot-password becomes a way to *set* a password on a
//! federated account — single sign-on as a local-account factory. Without the
//! one in `reset_password`, a token issued before an account was federated
//! still works afterwards.
//!
//! The stubs for the token store, the templates and the queue panic when
//! touched. That is the assertion: a refused request must not get far enough
//! to mint anything.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core_core::email_template::{EmailTemplate, EmailTemplateType};
use r_data_core_core::error::Result;
use r_data_core_core::password_reset_token::PasswordResetToken;
use r_data_core_persistence::{EmailTemplateRepositoryTrait, PasswordResetRepositoryTrait};
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::{FetchAndStageJob, ProcessRawItemJob, SendEmailJob};

use super::*;
use crate::oidc_test_fakes::{user, FakeUsers};

// ── stubs that must not be reached ──────────────────────────────────────────

/// A token store that fails the test if anything is written to it.
///
/// In `permissive` mode it records instead, which is how the positive case
/// checks that the guard let a local account through rather than merely that
/// it did not panic.
#[derive(Default)]
struct UnusedTokens {
    /// Seeded so `reset_password` can find a record to act on.
    seeded: Mutex<Option<PasswordResetToken>>,
    permissive: bool,
    reached: std::sync::atomic::AtomicBool,
}

impl UnusedTokens {
    fn permissive() -> Self {
        Self {
            permissive: true,
            ..Self::default()
        }
    }

    fn was_reached(&self) -> bool {
        self.reached.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Record that the flow got past the guard, or fail if it should not have.
    fn note(&self, what: &str) {
        assert!(self.permissive, "{what}");
        self.reached
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[async_trait]
impl PasswordResetRepositoryTrait for UnusedTokens {
    async fn insert_token(
        &self,
        _user_id: Uuid,
        _token_hash: &str,
        _expires_at: OffsetDateTime,
    ) -> Result<Uuid> {
        self.note("a reset token must never be minted for an SSO-provisioned account");
        Ok(Uuid::now_v7())
    }

    async fn find_by_token_hash(&self, _hash: &str) -> Result<Option<PasswordResetToken>> {
        Ok(self.seeded.lock().expect("lock").clone())
    }

    async fn find_latest_for_user(&self, _user_id: Uuid) -> Result<Option<PasswordResetToken>> {
        Ok(None)
    }

    async fn mark_used(&self, _id: Uuid) -> Result<()> {
        self.note("a refused reset must not consume a token");
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64> {
        Ok(0)
    }

    async fn delete_for_user(&self, _user_id: Uuid) -> Result<()> {
        self.note("a refused reset must not touch the token store");
        Ok(())
    }
}

struct UnusedTemplates;

#[async_trait]
impl EmailTemplateRepositoryTrait for UnusedTemplates {
    async fn list_all(&self) -> Result<Vec<EmailTemplate>> {
        Ok(vec![])
    }
    async fn list_by_type(&self, _t: EmailTemplateType) -> Result<Vec<EmailTemplate>> {
        Ok(vec![])
    }
    async fn get_by_uuid(&self, _uuid: Uuid) -> Result<Option<EmailTemplate>> {
        Ok(None)
    }
    async fn get_by_slug(&self, _slug: &str) -> Result<Option<EmailTemplate>> {
        // No template, so a request that gets this far ends in an error. The
        // refusal being tested happens earlier, at the token store, so this
        // does not need to panic — and must not, or the positive case cannot
        // reach the point it is asserting about.
        Ok(None)
    }
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        _name: &str,
        _slug: &str,
        _template_type: EmailTemplateType,
        _subject: &str,
        _body_html: &str,
        _body_text: &str,
        _variables: serde_json::Value,
        _created_by: Uuid,
    ) -> Result<Uuid> {
        unreachable!()
    }
    #[allow(clippy::too_many_arguments)]
    async fn update(
        &self,
        _uuid: Uuid,
        _name: Option<&str>,
        _subject: &str,
        _body_html: &str,
        _body_text: &str,
        _variables: serde_json::Value,
        _updated_by: Uuid,
    ) -> Result<()> {
        unreachable!()
    }
    async fn delete(&self, _uuid: Uuid) -> Result<()> {
        unreachable!()
    }
}

struct UnusedQueue;

#[async_trait]
impl JobQueue for UnusedQueue {
    async fn enqueue_fetch(&self, _job: FetchAndStageJob) -> Result<()> {
        unreachable!()
    }
    async fn enqueue_process(&self, _job: ProcessRawItemJob) -> Result<()> {
        unreachable!()
    }
    async fn enqueue_email(&self, _job: SendEmailJob) -> Result<()> {
        panic!("no reset email should be queued for an SSO-provisioned account");
    }
    async fn blocking_pop_email(&self) -> Result<SendEmailJob> {
        unreachable!()
    }
}

// ── fixtures ────────────────────────────────────────────────────────────────

/// An SSO-provisioned account, as `first_sign_in` leaves one.
fn federated(uuid: Uuid) -> r_data_core_core::admin_user::AdminUser {
    let mut u = user(uuid, "ada", true);
    u.is_sso_provisioned = true;
    u
}

fn service(users: Arc<FakeUsers>, tokens: Arc<UnusedTokens>) -> PasswordResetService {
    let mail = crate::mail::MailService::new(&r_data_core_core::config::SmtpConfig {
        host: "localhost".to_string(),
        port: 25,
        username: None,
        password: None,
        from_address: "noreply@example.com".to_string(),
        from_name: None,
        tls: false,
    })
    .expect("a transport is built, not connected");

    PasswordResetService::new(
        tokens,
        users,
        Arc::new(UnusedTemplates),
        Arc::new(UnusedQueue),
        Arc::new(mail),
        0,
        "https://rdc.example.com".to_string(),
    )
}

// ── tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_reset_is_issued_for_an_sso_provisioned_account() {
    let uuid = Uuid::now_v7();
    let users = Arc::new(FakeUsers::with(vec![federated(uuid)]));

    let outcome = service(users, Arc::new(UnusedTokens::default()))
        .request_reset("ada@example.com")
        .await;

    assert!(
        matches!(outcome, Ok(None)),
        "forgot-password must not become a way to set a password on a federated account"
    );
}

#[tokio::test]
async fn the_refusal_is_silent_just_like_an_unknown_address() {
    let users = Arc::new(FakeUsers::with(vec![federated(Uuid::now_v7())]));
    let svc = service(users, Arc::new(UnusedTokens::default()));

    let federated_outcome = svc.request_reset("ada@example.com").await;
    let unknown_outcome = svc.request_reset("nobody@example.com").await;

    // Answering differently would tell a stranger which accounts are
    // federated, which is an oracle this endpoint deliberately does not offer.
    assert!(matches!(federated_outcome, Ok(None)));
    assert!(matches!(unknown_outcome, Ok(None)));
}

#[tokio::test]
async fn a_local_account_still_reaches_the_token_store() {
    // The guard must be narrow. If this ever fails, password recovery is
    // broken for everyone, which is a far bigger outage than the bug it fixes.
    let uuid = Uuid::now_v7();
    let users = Arc::new(FakeUsers::with(vec![user(uuid, "grace", true)]));
    let tokens = Arc::new(UnusedTokens::permissive());

    // It fails later, at the template stub, which is fine: what is being
    // asserted is that it got *past* the guard.
    let _ = service(users, Arc::clone(&tokens))
        .request_reset("grace@example.com")
        .await;

    assert!(
        tokens.was_reached(),
        "a local account must still be offered recovery"
    );
}

#[tokio::test]
async fn a_token_predating_federation_cannot_be_consumed() {
    let uuid = Uuid::now_v7();
    let users = Arc::new(FakeUsers::with(vec![federated(uuid)]));

    // A token minted while the account was still local, honoured only if the
    // second guard is missing.
    let tokens = Arc::new(UnusedTokens::default());
    *tokens.seeded.lock().expect("lock") = Some(PasswordResetToken {
        id: Uuid::now_v7(),
        user_id: uuid,
        token_hash: hex::encode(Sha256::digest(b"still-valid")),
        expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
        created_at: OffsetDateTime::now_utc(),
        used_at: None,
    });

    let outcome = service(users, tokens)
        .reset_password("still-valid", "a-new-password")
        .await;

    assert!(
        outcome.is_err(),
        "a token issued before the account was federated must stop working"
    );
}
