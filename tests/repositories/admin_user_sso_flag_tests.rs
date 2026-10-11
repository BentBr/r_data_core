#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The one column whose absence must be an error.
//!
//! Every password-path guard trips only on `is_sso_provisioned = true`. A
//! query that forgot the column and defaulted it to `false` would quietly
//! turn a federated account back into a local one. Reading the row must
//! fail instead.

use r_data_core_core::admin_user::AdminUser;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use sqlx::FromRow as _;

#[tokio::test]
#[serial]
async fn a_row_without_the_sso_flag_does_not_read_as_a_local_account() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear");
    let uuid = create_test_admin_user(&pool).await.expect("an admin");

    let narrow = sqlx::query(
        "SELECT uuid, username, email, password_hash, first_name, last_name, is_active, \
         created_at, updated_at, status, failed_login_attempts, locked_until, super_admin, \
         last_login FROM admin_users WHERE uuid = $1",
    )
    .bind(uuid)
    .fetch_one(&pool.pool)
    .await
    .expect("the row");

    assert!(
        AdminUser::from_row(&narrow).is_err(),
        "a query that omits is_sso_provisioned must not produce a user"
    );

    let full = sqlx::query("SELECT * FROM admin_users WHERE uuid = $1")
        .bind(uuid)
        .fetch_one(&pool.pool)
        .await
        .expect("the row");
    assert!(
        !AdminUser::from_row(&full)
            .expect("a full row reads")
            .is_sso_provisioned
    );
}
