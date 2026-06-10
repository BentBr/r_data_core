---
name: core-crate
description: Domain models, configuration, cache abstraction, field types, versioning, and foundational types in crates/core/
---

# Core Crate (`r_data_core_core`)

**Path**: `crates/core/`
**Role**: Foundation layer — pure domain models, no internal crate dependencies.

## Key Modules

| Module | Responsibility |
|--------|----------------|
| `config/` | Configuration loading: `AppConfig`, `WorkerConfig`, `MaintenanceConfig`, `DatabaseConfig`, `CacheConfig`, `QueueConfig`, `LicenseConfig` |
| `domain/` | Core domain models: `DynamicEntity`, `AbstractRDataEntity` |
| `entity_definition/` | Entity schema definitions with field types, validation rules, UI settings |
| `field/` | Field type definitions, constraints, and value handling |
| `admin_user/` | Admin user models and operations |
| `admin_jwt/` | JWT generation/validation for admin auth |
| `entity_jwt/` | JWT generation/validation for entity/public auth |
| `cache/` | Cache abstraction (Redis + in-memory backends via `CacheManager`) |
| `permissions/` | Permission and role models |
| `public_api/` | Public API models |
| `refresh_token/` | Refresh token models |
| `settings/` | System settings management |
| `versioning/` | Entity versioning support |
| `crypto/` | Cryptographic utilities |
| `maintenance/` | Maintenance utilities |
| `error.rs` | Custom error types with `thiserror` |

## Key Exports

- `DynamicEntity` — the central domain model for flexible runtime entities
- Configuration structs loaded from environment variables
- `CacheManager` — dual-backend cache (Redis / in-memory LRU)
- JWT claim types and token generation
- Field type system (definition, constraints, values)

## Dependencies

serde, sqlx (types only), uuid, time, redis, lru, cron, jsonwebtoken, argon2, chrono, sha2, base64, regex, async-trait, tokio

## Patterns

- Pure domain layer — no I/O or framework dependencies
- Configuration via environment variables with dotenv support
- Error types using `thiserror` for ergonomic `?` propagation
- Cache abstraction hides backend (Redis vs in-memory) behind trait
