---
name: license-crate
description: License verification, JWT-based licensing, and CLI tool in crates/license/
---

# License Crate (`r_data_core_license`)

**Path**: `crates/license/`
**Role**: License verification and management.
**Depends on**: core only

## Key Modules

| Module | Responsibility |
|--------|----------------|
| `models.rs` | License data models: `LicenseType`, `LicenseClaims` |
| `jwt.rs` | License JWT creation and verification |
| `api.rs` | License verification API client |
| `tool_service.rs` | `LicenseToolService` — check, create, display results |

## Binary

`license_tool` — Command-line license management utility (uses clap)

## Key Exports

- License creation/verification functions
- `LicenseToolService`
- `LicenseCheckResult`
- JWT operations for license tokens

## Caching

License verification results are cached:
- `LICENSE_CACHE_KEY_PREFIX` — cache key namespace
- `LICENSE_CACHE_TTL_SECS` — cache TTL

## Patterns

- JWT-based license claims
- API client for remote license verification
- Cache integration to avoid repeated verification
