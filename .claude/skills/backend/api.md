---
name: api-crate
description: Actix-web HTTP API layer — middleware, authentication, admin/public endpoints, OpenAPI docs in crates/api/
---

# API Crate (`r_data_core_api`)

**Path**: `crates/api/`
**Role**: HTTP boundary layer using Actix-web. Depends on all other crates.

## Key Modules

| Module | Responsibility |
|--------|----------------|
| `api_state.rs` / `api_state_impl.rs` | Shared application state (`ApiState`, `ApiStateTrait`) |
| `middleware/` | HTTP middleware: auth, error handling |
| `auth/` | Permission checks, guards, API key extraction |
| `admin/` | Admin API endpoint handlers |
| `public/` | Public API endpoint handlers |
| `docs/` | OpenAPI/Swagger documentation (utoipa) |
| `health.rs` | Health check endpoints |
| `response.rs` | Standardized `ApiResponse` wrapper |
| `models.rs` | Request/response DTOs |
| `token_service.rs` | JWT token management |
| `client_ip.rs` | Client address behind trusted proxies (`X-Forwarded-For`) |
| `query/` | Query parameter handling |

## Middleware Stack

1. Logger
2. CORS
3. Authentication (JWT / API Key / Combined)
4. Error handling

## Authentication Modules

| File | Purpose |
|------|---------|
| `middleware/base_auth.rs` | Basic authentication |
| `middleware/jwt_auth.rs` | JWT authentication |
| `middleware/api_auth.rs` | API key authentication |
| `middleware/combined_auth.rs` | Multi-method auth |
| `auth/permission_check.rs` | Permission verification |
| `auth/permission_required.rs` | Permission guard macros |

## Admin Endpoints (`/admin/api/v1/`)

Organized in submodules: `api_keys/`, `auth/`, `entity_definitions/`, `workflows/`, `users/`, `permissions/`, `system/`, `meta/`, `dsl/`

### Auth hardening (`admin/auth/`)

| Piece | Notes |
|-------|-------|
| `rate_limit.rs` | Per-client-IP throttle, separate `Login` / `Register` buckets, atomic `CacheManager::increment` |
| `routes/login.rs` | Password verified before any locked/inactive state is revealed; unknown users get a dummy Argon2 verify so timing does not leak account existence |
| `routes/helpers.rs` | Lockout persistence via `update_lockout_state(uuid, status, attempts, locked_until)` |

Keying comes from `client_ip::client_ip_key`, which only believes
`X-Forwarded-For` when the peer is in `TRUSTED_PROXIES`. Thresholds and windows
live in `r_data_core_core::config::SecurityConfig` (read from env, not `ApiConfig` —
adding a field there would touch ~60 test literals).

Unlocking is `PUT /admin/api/v1/users/{uuid}` with `status: "active"`, which
routes through `AdminUser::set_status` and clears the counter and expiry.

## Public Endpoints (`/api/v1/`)

Organized in submodules: `dynamic_entities/`, `entities/`, `workflows/`, `queries/`

## Key Exports

`ApiState`, `ApiStateWrapper`, `ApiConfiguration`, `ApiResponse`, `ApiStateTrait`

## Patterns

- Actix-web handlers with extractors
- OpenAPI documentation via utoipa
- `future_not_send` allows needed on handlers taking `HttpRequest`/`Multipart`
- Standardized response wrapping via `ApiResponse`
