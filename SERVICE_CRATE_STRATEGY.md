# Service Crate Migration Strategy

## Current State

Services are in `src/services/` and have the following dependencies:

### ✅ Already Using External Crates
- `r_data_core_core` - Domain models, error types, cache, config
- `r_data_core_persistence` - Repository traits and implementations

### ❌ Still Using Main Crate
- `crate::workflow` - Workflow types (`Workflow`, `WorkflowRepositoryTrait`)
- `crate::api::admin::workflows::models` - API request/response models
- `crate::entity::admin_user` - Should use `r_data_core_core::admin_user` and `r_data_core_persistence`
- `crate::config` - Should use `r_data_core_core::config`
- `crate::services` - Internal dependencies (fine, will be in same crate)

## Proposed Architecture

```
r_data_core_core
  └─ Domain models, error types, config, cache, permissions

r_data_core_persistence
  └─ Depends on: core
  └─ Repository implementations

r_data_core_services (NEW)
  └─ Depends on: core, persistence
  └─ Business logic services

r_data_core_api
  └─ Depends on: core, persistence, services
  └─ HTTP layer, routes, middleware

r_data_core (main)
  └─ Depends on: all above
  └─ Glues everything together, concrete ApiState
```

## Migration Steps

### Phase 1: Fix Dependencies (Can be done now)

1. **Fix entity imports:**
   - `crate::entity::admin_user` → `r_data_core_core::admin_user` + `r_data_core_persistence`
   - Already mostly done, just need to verify

2. **Fix config imports:**
   - `crate::config::CacheConfig` → `r_data_core_core::config::CacheConfig`
   - Update `entity_definition_service.rs`

3. **Move workflow types:**
   - Option A: Move `src/workflow/` to `crates/core/src/workflow/`
   - Option B: Create `crates/workflow/` crate
   - **Recommendation: Option A** (workflow is domain logic, belongs in core)

4. **Move API models:**
   - Already in progress - moving to `crates/api/src/admin/workflows/models.rs`
   - Services can depend on API crate for these models, OR
   - Create DTOs in services crate that API crate converts from/to

### Phase 2: Create Service Crate

1. **Create `crates/services/Cargo.toml`:**
```toml
[package]
name = "r_data_core_services"
version = "0.1.0"
edition = "2021"

[dependencies]
r_data_core_core = { path = "../core" }
r_data_core_persistence = { path = "../persistence" }
# Optionally for workflow models:
r_data_core_api = { path = "../api" }

# External dependencies
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.8.4", features = ["postgres", "uuid", "time"] }
uuid = { version = "1.6", features = ["v7", "serde"] }
log = "0.4"
anyhow = "1.0"
cron = "0.12"
```

2. **Create `crates/services/src/lib.rs`:**
```rust
#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod admin_user;
pub mod api_key;
pub mod dynamic_entity;
pub mod entity_definition;
pub mod permission_scheme;
pub mod settings;
pub mod version;
pub mod workflow;

// Re-exports
pub use admin_user::AdminUserService;
pub use api_key::ApiKeyService;
pub use dynamic_entity::DynamicEntityService;
pub use entity_definition::EntityDefinitionService;
pub use permission_scheme::PermissionSchemeService;
pub use settings::SettingsService;
pub use version::VersionService;
pub use workflow::WorkflowService;
```

3. **Migrate services one by one:**
   - Start with simple ones: `settings`, `version`, `permission_scheme`
   - Then: `admin_user`, `api_key`
   - Then: `entity_definition`, `dynamic_entity`
   - Finally: `workflow` (most complex)

### Phase 3: Update Main Crate

1. **Update `Cargo.toml`:**
```toml
r_data_core_services = { path = "crates/services" }
```

2. **Update imports in `src/main.rs` and routes:**
   - `crate::services::*` → `r_data_core_services::*`

3. **Remove `src/services/` directory**

## Benefits

1. **Clear separation of concerns:**
   - Core: Domain models
   - Persistence: Data access
   - Services: Business logic
   - API: HTTP layer
   - Main: Application setup

2. **No circular dependencies:**
   - Services depend on core + persistence
   - API depends on core + persistence + services
   - Main depends on all

3. **Better testability:**
   - Services can be tested independently
   - Can mock repositories easily

4. **Reusability:**
   - Services can be used by CLI tools, workers, etc.

## Challenges

1. **Workflow types:**
   - Need to decide: core crate or separate workflow crate?
   - **Recommendation: Move to core** since it's domain logic

2. **API models in services:**
   - `WorkflowService` uses `CreateWorkflowRequest`, `UpdateWorkflowRequest`
   - Options:
     a. Services depend on API crate (creates dependency: services → api)
     b. Create service-level DTOs, API converts
     c. Move request models to core (not ideal, they're API-specific)
   - **Recommendation: Option b** - Create service DTOs

3. **Internal service dependencies:**
   - `DynamicEntityService` uses `EntityDefinitionService`
   - Fine within same crate, just use relative imports

## Implementation Order

1. ✅ Fix entity/config imports (quick wins)
2. Move workflow types to core
3. Create service DTOs for workflow operations
4. Create `crates/services/` structure
5. Migrate simple services first
6. Migrate complex services
7. Update main crate
8. Remove old `src/services/`

## Alternative: Keep Services in Main Crate

If workflow migration is too complex, we could:
- Keep services in main crate
- But organize them better
- Use strict module boundaries
- This is acceptable if workflow types are tightly coupled to main crate

However, **service crate is the better long-term solution** for clean architecture.

