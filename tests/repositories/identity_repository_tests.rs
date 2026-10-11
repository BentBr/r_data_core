#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The identity table's two promises: an identity resolves to exactly one
//! account, and a subject already linked keeps its owner no matter who asks.
//!
//! `setup_test_db` gives each test its own schema; every test also uses its
//! own issuer, as belt-and-braces against any future shared state.

use r_data_core_persistence::{IdentityRepository, IdentityRepositoryTrait};
use r_data_core_test_support::{create_test_admin_user, setup_test_db};
use uuid::Uuid;

fn issuer() -> String {
    format!(
        "https://idp.example.test/realms/{}",
        Uuid::now_v7().simple()
    )
}

async fn identity_row_count(pool: &sqlx::PgPool, provider: &str, subject: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_user_identities WHERE provider = $1 AND subject = $2",
    )
    .bind(provider)
    .bind(subject)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn email_of(pool: &sqlx::PgPool, user: Uuid) -> String {
    sqlx::query_scalar("SELECT email FROM admin_users WHERE uuid = $1")
        .bind(user)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn an_unknown_identity_resolves_to_nobody() {
    let db = setup_test_db().await;
    let repo = IdentityRepository::new(db.pool.clone());

    assert!(repo
        .find_user_by_identity(&issuer(), "nobody")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn linking_then_finding_round_trips() {
    let db = setup_test_db().await;
    let repo = IdentityRepository::new(db.pool.clone());
    let user = create_test_admin_user(&db.pool).await.unwrap();
    let issuer = issuer();

    let resolved = repo.link_identity(&issuer, "ada", user).await.unwrap();

    assert_eq!(resolved, user);
    assert_eq!(
        repo.find_user_by_identity(&issuer, "ada").await.unwrap(),
        Some(user)
    );
    assert_eq!(
        repo.find_identities_for_user(user).await.unwrap(),
        vec![(issuer, "ada".to_string())]
    );
}

#[tokio::test]
async fn a_subject_already_linked_keeps_its_owner() {
    let db = setup_test_db().await;
    let repo = IdentityRepository::new(db.pool.clone());
    let first = create_test_admin_user(&db.pool).await.unwrap();
    let second = create_test_admin_user(&db.pool).await.unwrap();
    let issuer = issuer();
    repo.link_identity(&issuer, "ada", first).await.unwrap();

    let resolved = repo.link_identity(&issuer, "ada", second).await.unwrap();

    assert_eq!(
        resolved, first,
        "last-writer-wins here would be an account takeover"
    );
    assert_eq!(identity_row_count(&db.pool, &issuer, "ada").await, 1);
}

#[tokio::test]
async fn two_concurrent_first_logins_converge_on_one_account() {
    let db = setup_test_db().await;
    let repo_a = IdentityRepository::new(db.pool.clone());
    let repo_b = IdentityRepository::new(db.pool.clone());
    let user_a = create_test_admin_user(&db.pool).await.unwrap();
    let user_b = create_test_admin_user(&db.pool).await.unwrap();
    let issuer = issuer();

    let (a, b) = tokio::join!(
        repo_a.link_identity(&issuer, "race", user_a),
        repo_b.link_identity(&issuer, "race", user_b)
    );

    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a, b, "both callers must see the same owner");
    assert!(a == user_a || a == user_b);
    assert_eq!(identity_row_count(&db.pool, &issuer, "race").await, 1);
}

#[tokio::test]
async fn the_same_subject_at_another_issuer_is_another_identity() {
    let db = setup_test_db().await;
    let repo = IdentityRepository::new(db.pool.clone());
    let first = create_test_admin_user(&db.pool).await.unwrap();
    let second = create_test_admin_user(&db.pool).await.unwrap();

    repo.link_identity(&issuer(), "ada", first).await.unwrap();
    let other = repo.link_identity(&issuer(), "ada", second).await.unwrap();

    assert_eq!(other, second);
}

#[tokio::test]
async fn touching_last_login_records_the_time() {
    let db = setup_test_db().await;
    let repo = IdentityRepository::new(db.pool.clone());
    let user = create_test_admin_user(&db.pool).await.unwrap();
    let issuer = issuer();
    repo.link_identity(&issuer, "ada", user).await.unwrap();

    repo.touch_last_login(&issuer, "ada").await.unwrap();

    let last: Option<time::OffsetDateTime> = sqlx::query_scalar(
        "SELECT last_login_at FROM admin_user_identities WHERE provider = $1 AND subject = $2",
    )
    .bind(&issuer)
    .bind("ada")
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(last.is_some());
}

#[tokio::test]
async fn marking_provisioned_flips_the_flag_and_hides_the_account_from_email_linking() {
    let db = setup_test_db().await;
    let repo = IdentityRepository::new(db.pool.clone());
    let user = create_test_admin_user(&db.pool).await.unwrap();
    let email = email_of(&db.pool, user).await;
    assert_eq!(
        repo.find_local_user_by_email(&email.to_uppercase())
            .await
            .unwrap(),
        Some(user),
        "case-insensitive"
    );

    repo.mark_sso_provisioned(user).await.unwrap();

    let flagged: bool =
        sqlx::query_scalar("SELECT is_sso_provisioned FROM admin_users WHERE uuid = $1")
            .bind(user)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(flagged);
    assert!(
        repo.find_local_user_by_email(&email)
            .await
            .unwrap()
            .is_none(),
        "an SSO account is never adopted by address"
    );
}

#[tokio::test]
async fn deleting_the_user_removes_the_identity() {
    let db = setup_test_db().await;
    let repo = IdentityRepository::new(db.pool.clone());
    let user = create_test_admin_user(&db.pool).await.unwrap();
    let issuer = issuer();
    repo.link_identity(&issuer, "ada", user).await.unwrap();

    sqlx::query("DELETE FROM admin_users WHERE uuid = $1")
        .bind(user)
        .execute(&db.pool)
        .await
        .unwrap();

    assert_eq!(
        identity_row_count(&db.pool, &issuer, "ada").await,
        0,
        "ON DELETE CASCADE"
    );
    assert!(repo
        .find_user_by_identity(&issuer, "ada")
        .await
        .unwrap()
        .is_none());
}
