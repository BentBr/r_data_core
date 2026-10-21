#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The role snapshot a single-sign-on session carries on its refresh chain.
//!
//! An SSO session's authority is the roles its sign-in mapped to (D1), and
//! those are never stored on the account. The refresh token therefore carries
//! them in its `device_info`, alongside a marker and the original sign-in
//! time. Every refresh copies `device_info` forward, so the whole chain is
//! bound to one sign-in: the snapshot is the authority, and the chain lives at
//! most [`REFRESH_TOKEN_EXPIRY_SECONDS`] from that sign-in however often it is
//! refreshed (D5).

use serde_json::Value;
use time::OffsetDateTime;

use crate::admin::auth::routes::helpers::load_user_roles;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use r_data_core_core::admin_jwt::REFRESH_TOKEN_EXPIRY_SECONDS;
use r_data_core_core::admin_user::AdminUser;
use r_data_core_core::permissions::role::Role;
use r_data_core_core::refresh_token::RefreshToken;
use r_data_core_persistence::{
    AdminUserRepository, RefreshTokenRepository, RefreshTokenRepositoryTrait,
};

/// Key in a refresh token's `device_info` holding the role UUIDs an SSO
/// sign-in mapped to.
pub const SSO_ROLE_UUIDS: &str = "sso_role_uuids";
/// Key in a refresh token's `device_info` marking it as an SSO session.
pub const SSO_MARKER: &str = "sso";
/// Key in a refresh token's `device_info` holding the original sign-in time.
pub const LOGIN_TIME: &str = "login_time";

/// How a session began, which decides what its refresh token carries.
///
/// Passed explicitly rather than read from the account's
/// `is_sso_provisioned` flag: an account adopted by email (D2) stays a local
/// account by flag, but a session it obtained through the provider is still
/// an SSO session and must be bound to its mapped roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOrigin {
    /// Password login; refreshes read the account's stored roles.
    Local,
    /// Single sign-on; refreshes use the roles snapshotted at sign-in.
    Sso,
}

/// Add the SSO marker and role snapshot to a refresh token's `device_info`.
pub(super) fn mark_sso(device_info: &mut Value, roles: &[Role]) {
    device_info[SSO_MARKER] = Value::Bool(true);
    device_info[SSO_ROLE_UUIDS] =
        serde_json::json!(roles.iter().map(|r| r.base.uuid).collect::<Vec<_>>());
}

/// Whether a refresh token belongs to an SSO session.
///
/// The marker is authoritative; a bare snapshot counts too, so a row written
/// before the marker existed is still treated as SSO rather than falling back
/// to stored roles.
pub(super) fn is_sso_session(device_info: Option<&Value>) -> bool {
    device_info.is_some_and(|info| {
        info.get(SSO_MARKER).and_then(Value::as_bool) == Some(true)
            || info.get(SSO_ROLE_UUIDS).is_some()
    })
}

/// Whether an SSO chain has outlived its absolute lifetime.
///
/// Measured from the original sign-in, not from the last refresh. A missing
/// or unreadable sign-in time counts as expired: an unbounded chain is
/// exactly what this check exists to prevent.
pub(super) fn sso_chain_expired(device_info: Option<&Value>, now: OffsetDateTime) -> bool {
    let Some(login_time) = device_info
        .and_then(|info| info.get(LOGIN_TIME))
        .and_then(|v| serde_json::from_value::<OffsetDateTime>(v.clone()).ok())
    else {
        return true;
    };
    let lifetime =
        time::Duration::seconds(i64::try_from(REFRESH_TOKEN_EXPIRY_SECONDS).unwrap_or(0));
    now > login_time + lifetime
}

/// The roles an SSO refresh token was issued with.
///
/// Still-existing roles only, and never a super-admin one: sign-in mapping
/// drops those (`map_roles`), and a role flipped to super-admin after sign-in
/// must not become super-admin through a refresh.
///
/// `None` when the token carries no snapshot, which the caller must treat as
/// "sign in again" rather than mint an empty session.
pub async fn sso_roles_from_refresh(
    device_info: Option<&Value>,
    data: &ApiStateWrapper,
) -> Option<Vec<Role>> {
    let uuids = device_info?.get(SSO_ROLE_UUIDS)?.as_array()?;
    let mut roles = Vec::new();
    for uuid in uuids
        .iter()
        .filter_map(|v| v.as_str().and_then(|s| s.parse::<uuid::Uuid>().ok()))
    {
        match data.role_service().get_role(uuid).await {
            Ok(Some(role)) if role.super_admin => {
                log::warn!("Mapped role {uuid} is now super-admin; dropped on refresh");
            }
            Ok(Some(role)) => roles.push(role),
            Ok(None) => log::debug!("Mapped role {uuid} no longer exists"),
            Err(e) => log::warn!("Failed to load mapped role {uuid}: {e}"),
        }
    }
    Some(roles)
}

/// The roles a refreshed session is minted from, or `None` to refuse the
/// refresh with a generic 401 (the reason is logged).
///
/// An SSO session continues with the roles its sign-in mapped to — whatever
/// the account's flag says, so an email-adopted account never falls back to
/// its stored roles — until the chain outlives its sign-in. A federated
/// account presenting a row without a snapshot is refused: the row predates
/// the snapshot, or was not issued by a sign-in, and either way the answer is
/// "sign in again". Everything else reads the account's stored roles.
pub(super) async fn roles_for_refresh(
    refresh_token: &RefreshToken,
    user: &AdminUser,
    data: &ApiStateWrapper,
    admin_repo: &AdminUserRepository,
) -> Option<Vec<Role>> {
    let device_info = refresh_token.device_info.as_ref();
    if is_sso_session(device_info) {
        if sso_chain_expired(device_info, OffsetDateTime::now_utc()) {
            log::warn!("SSO refresh chain outlived its sign-in; refusing");
            let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());
            if let Err(e) = refresh_repo.revoke_by_id(refresh_token.id).await {
                log::error!("Failed to revoke an expired SSO refresh token: {e:?}");
            }
            return None;
        }
        let roles = sso_roles_from_refresh(device_info, data).await;
        if roles.is_none() {
            log::warn!("SSO refresh token carries no role snapshot; refusing");
        }
        return roles;
    }
    if user.is_sso_provisioned {
        log::warn!("SSO account presented a refresh token without a role snapshot; refusing");
        return None;
    }
    Some(load_user_roles(user, data, admin_repo).await)
}

#[cfg(test)]
mod tests {
    use super::{is_sso_session, sso_chain_expired, LOGIN_TIME, SSO_MARKER, SSO_ROLE_UUIDS};
    use serde_json::json;
    use time::{Duration, OffsetDateTime};

    fn info_logged_in(ago: Duration) -> serde_json::Value {
        json!({ LOGIN_TIME: OffsetDateTime::now_utc() - ago, SSO_MARKER: true })
    }

    #[test]
    fn a_chain_within_thirty_days_of_sign_in_is_live() {
        let info = info_logged_in(Duration::days(29));
        assert!(!sso_chain_expired(Some(&info), OffsetDateTime::now_utc()));
    }

    #[test]
    fn a_chain_older_than_thirty_days_has_expired() {
        let info = info_logged_in(Duration::days(31));
        assert!(sso_chain_expired(Some(&info), OffsetDateTime::now_utc()));
    }

    #[test]
    fn a_chain_without_a_sign_in_time_has_expired() {
        assert!(sso_chain_expired(
            Some(&json!({ SSO_MARKER: true })),
            OffsetDateTime::now_utc()
        ));
        assert!(sso_chain_expired(
            Some(&json!({ LOGIN_TIME: "not a time" })),
            OffsetDateTime::now_utc()
        ));
        assert!(sso_chain_expired(None, OffsetDateTime::now_utc()));
    }

    #[test]
    fn the_marker_or_a_snapshot_identifies_an_sso_session() {
        assert!(is_sso_session(Some(&json!({ SSO_MARKER: true }))));
        assert!(is_sso_session(Some(&json!({ SSO_ROLE_UUIDS: [] }))));
        assert!(!is_sso_session(Some(&json!({ "user_agent": "login" }))));
        assert!(!is_sso_session(Some(&json!({ SSO_MARKER: false }))));
        assert!(!is_sso_session(None));
    }
}
