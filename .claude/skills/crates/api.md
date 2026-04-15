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

## Public Endpoints (`/api/v1/`)

Organized in submodules: `dynamic_entities/`, `entities/`, `workflows/`, `queries/`

## Key Exports

`ApiState`, `ApiStateWrapper`, `ApiConfiguration`, `ApiResponse`, `ApiStateTrait`

## Patterns

- Actix-web handlers with extractors
- OpenAPI documentation via utoipa
- `future_not_send` allows needed on handlers taking `HttpRequest`/`Multipart`
- Standardized response wrapping via `ApiResponse`
