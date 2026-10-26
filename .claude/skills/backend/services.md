---
name: services-crate
description: Business logic layer — service implementations, adapters, validation in crates/services/
---

# Services Crate (`r_data_core_services`)

**Path**: `crates/services/`
**Role**: Business logic layer — orchestrates domain operations.
**Depends on**: core, license, oidc-http, persistence, workflow

## Key Modules

| Module | Responsibility |
|--------|----------------|
| `dynamic_entity/` | Entity CRUD service, validation, filtering |
| `workflow/` | Workflow orchestration, item processing, adapters |
| `entity_definition/` | Entity schema management |
| `admin_user.rs` | Admin user service |
| `api_key/` | API key service with caching |
| `auth.rs` | Authentication service |
| `role.rs` | Role service |
| `license/` | License service |
| `cache.rs` | Cache management service |
| `statistics/` | Statistics collection and reporting |
| `dashboard_stats.rs` | Dashboard metrics computation |
| `version.rs` | Version management service |
| `settings.rs` | System settings service |
| `worker.rs` | Worker task service (reconciliation logic) |
| `bootstrap.rs` | Service initialization |
| `adapters/` | Repository pattern adapters |
| `oidc_runtime.rs` | `OidcRuntime`: verifies provider tokens, resolves `(issuer, subject)` to a user + mapped roles (`Resolution`), caches the resolution |
| `oidc_flow.rs` | `OidcFlow`: browser authorization-code flow (PKCE, state, nonce, `return_to`), yields a `CompletedLogin`; `OidcServices` bundles runtime + flow |
| `oidc_provisioning.rs` | `OidcProvisioningService`: first-login provisioning, role mapping, optional verified-email linking; `ensure_may_sign_in` |
| `oidc_test_fakes.rs` | In-memory identity/user/role fakes for tests |
| `workflow/dry_run/` | Dry-run: in-memory entity overlay (`repository.rs`), per-step trace, suppressed side effects (`service.rs`) |
| `query_validation.rs` | Query parameter validation |

## Dynamic Entity Service

```
dynamic_entity/
├── crud.rs        # Create, read, update, delete
├── validation.rs  # Entity validation rules
├── filtering.rs   # Advanced filtering
└── tests.rs       # Service tests
```

## Adapter Pattern

```
adapters/
├── main_adapters.rs   # Core repository adapters
└── api_key_adapter.rs # API key adapter
```

Services receive repository implementations through adapter injection, keeping business logic decoupled from persistence.

## Patterns

- Service adapters for repository injection (no direct DB access)
- Validation composition in dedicated modules
- Bootstrap module wires services together
- `async-trait` for async service interfaces
