#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! Trait-object dispatch for the API key role methods.
//!
//! `ApiKeyRepository` defines `get_api_key_roles`/`assign_role`/`unassign_role`/
//! `update_api_key_roles` both as inherent methods (used internally by
//! `find_api_key_for_auth` etc.) and as `ApiKeyRepositoryTrait` pass-through
//! wrappers (`Self::method(self, ...)`) for trait-object callers. Calling
//! through the concrete `ApiKeyRepository` type always resolves to the
//! inherent methods, so the trait wrapper bodies are only exercised via a
//! `dyn ApiKeyRepositoryTrait` (or generic) caller — the shape every real
//! caller (API handlers) actually uses.

use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepository;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

async fn create_test_role(pool: &PgPool, creator: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO roles (name, created_by, published) VALUES ($1, $2, true) RETURNING uuid",
    )
    .bind(random_string("role"))
    .bind(creator)
    .fetch_one(pool)
    .await
    .expect("failed to create test role")
}

#[tokio::test]
#[serial]
async fn test_trait_object_role_methods_round_trip() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let concrete = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;
    let (key_uuid, _) = concrete
        .create_new_api_key(&random_string("trait_obj_key"), "d", user_uuid, 30)
        .await?;

    // Upcast to the trait object so the pass-through wrapper bodies (not the
    // inherent methods) are the code under test.
    let repo: Box<dyn ApiKeyRepositoryTrait> = Box::new(concrete);

    let role1 = create_test_role(&pool.pool, user_uuid).await;
    let role2 = create_test_role(&pool.pool, user_uuid).await;

    let before = repo.get_api_key_roles(key_uuid).await?;
    assert!(before.is_empty(), "no roles assigned yet");

    repo.assign_role(key_uuid, role1).await?;
    repo.assign_role(key_uuid, role2).await?;

    let after_assign = repo.get_api_key_roles(key_uuid).await?;
    assert_eq!(after_assign.len(), 2);
    assert!(after_assign.contains(&role1));
    assert!(after_assign.contains(&role2));

    repo.unassign_role(key_uuid, role1).await?;
    let after_unassign = repo.get_api_key_roles(key_uuid).await?;
    assert_eq!(after_unassign.len(), 1);
    assert!(after_unassign.contains(&role2));

    let role3 = create_test_role(&pool.pool, user_uuid).await;
    repo.update_api_key_roles(key_uuid, &[role3]).await?;
    let after_replace = repo.get_api_key_roles(key_uuid).await?;
    assert_eq!(after_replace.len(), 1);
    assert!(after_replace.contains(&role3));
    assert!(!after_replace.contains(&role2));

    Ok(())
}
