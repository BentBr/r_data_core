#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Turning an identity provider's groups into `RDataCore` roles.
//!
//! Two rules here are not negotiable, and both are deliberately awkward.
//!
//! **An unmapped user is rejected.** Not admitted with no permissions —
//! rejected. Admitting them creates a real account with a real session that
//! merely happens to be able to do nothing *today*, and that changes the
//! moment someone configures a broad default.
//!
//! **Claim mapping never yields super-admin.** A group name is a string
//! controlled by whoever administers the identity provider, who is often not
//! the person who administers this system. Mapping one onto a super-admin role
//! would hand over the instance to a string match, so those roles are filtered
//! out and the drop is logged.

use super::config::OidcConfig;
use super::keys::OidcClaims;
use crate::permissions::role::Role;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MappingError {
    #[error(
        "no group on this identity maps to a role, and no default role is configured. \
         Set RDC_OIDC_DEFAULT_ROLE to admit unmapped users, or add a mapping."
    )]
    NoMappedRole,
}

/// Resolve the roles an identity should hold.
///
/// # Errors
/// Returns `MappingError::NoMappedRole` when nothing matches and no default is
/// configured — the deny-by-default case.
pub fn map_roles(
    claims: &OidcClaims,
    config: &OidcConfig,
    available: &[Role],
) -> Result<Vec<Role>, MappingError> {
    let mut mapped: Vec<Role> = claims
        .groups
        .iter()
        .filter_map(|group| config.role_map.get(group))
        .filter_map(|role_name| find_role(available, role_name))
        .cloned()
        .collect();

    if mapped.is_empty() {
        let default = config
            .default_role
            .as_ref()
            .and_then(|name| find_role(available, name))
            .ok_or(MappingError::NoMappedRole)?;
        mapped.push(default.clone());
    }

    let before = mapped.len();
    mapped.retain(|role| !role.super_admin);
    if mapped.len() != before {
        log::warn!(
            "OIDC role mapping matched a super-admin role and dropped it. Elevation to \
             super-admin is deliberately not grantable by claim mapping; grant it inside \
             RDataCore instead."
        );
    }

    // Dropping the only match leaves nothing, which must deny rather than
    // admit with an empty role set.
    if mapped.is_empty() {
        return Err(MappingError::NoMappedRole);
    }

    dedupe(&mut mapped);
    Ok(mapped)
}

/// Find a role by name.
///
/// A mapping naming a role that does not exist is a configuration error, but
/// not a fatal one: it is reported and the identity falls through to the same
/// deny-by-default path as an unmapped user, rather than being admitted.
fn find_role<'a>(available: &'a [Role], name: &str) -> Option<&'a Role> {
    let found = available.iter().find(|role| role.name == name);
    if found.is_none() {
        log::warn!(
            "OIDC role map names role '{name}', which does not exist. The mapping has no \
             effect; check RDC_OIDC_ROLE_MAP against the roles this instance defines."
        );
    }
    found
}

/// Remove duplicates, keeping first-seen order.
///
/// Several groups commonly map to the same role, and a user holding it twice
/// would appear in audit output as though something odd had happened.
fn dedupe(roles: &mut Vec<Role>) {
    let mut seen: Vec<String> = Vec::with_capacity(roles.len());
    roles.retain(|role| {
        if seen.contains(&role.name) {
            false
        } else {
            seen.push(role.name.clone());
            true
        }
    });
}

#[cfg(test)]
mod tests;
