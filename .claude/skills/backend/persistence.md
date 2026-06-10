---
name: persistence-crate
description: SQLx repository implementations, query patterns, and migration service in crates/persistence/
---

# Persistence Crate (`r_data_core_persistence`)

**Path**: `crates/persistence/`
**Role**: Data layer — SQLx-based repository implementations.
**Depends on**: core, workflow

## Key Modules

| Module | Responsibility |
|--------|----------------|
| `repository.rs` | Base `EntityRepository` trait and `PgPoolExtension` |
| `dynamic_entity_repository/` | Entity CRUD (create, query, filter, update) |
| `dynamic_entity_query_repository/` | Complex query operations |
| `dynamic_entity_public_repository/` | Public-facing entity queries |
| `dynamic_entity_versioning.rs` | Entity version history tracking |
| `entity_definition_repository.rs` | Entity schema CRUD |
| `entity_definition_versioning_repository.rs` | Schema version tracking |
| `workflow_repository/` | Workflow CRUD, runs, raw items |
| `workflow_run_repository.rs` | Workflow execution history |
| `workflow_versioning_repository.rs` | Workflow version tracking |
| `admin_user_repository.rs` | Admin user CRUD |
| `api_key_repository.rs` | API key management |
| `refresh_token_repository.rs` | Refresh token persistence |
| `role_repository.rs` | Role management |
| `version_repository.rs` | Generic entity version tracking |
| `statistics_repository.rs` | Statistics data persistence |
| `dashboard_stats_repository.rs` | Dashboard metrics |
| `settings_repository.rs` | System settings persistence |
| `migration_service.rs` | Database migration tracking |

## Binaries

| Binary | Purpose |
|--------|---------|
| `apply_schema` | Apply entity schema changes to database |
| `run_migrations` | Run SQLx database migrations |

## Dynamic Entity Repository Structure

```
dynamic_entity_repository/
├── create.rs   # Entity insertion
├── query.rs    # Retrieval (by type, uuid, parent, filters)
├── filter.rs   # Entity filtering/browsing
└── update.rs   # Entity updates
```

## Patterns

- Trait-based repositories for testability
- `PgPoolExtension` for shared pool utilities
- Compile-time query verification via SQLx (requires running Postgres)
- After schema changes: `cargo sqlx prepare --workspace -- --all-targets`
