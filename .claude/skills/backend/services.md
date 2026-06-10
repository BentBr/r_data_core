---
name: services-crate
description: Business logic layer — service implementations, adapters, validation in crates/services/
---

# Services Crate (`r_data_core_services`)

**Path**: `crates/services/`
**Role**: Business logic layer — orchestrates domain operations.
**Depends on**: core, persistence, workflow, license

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
