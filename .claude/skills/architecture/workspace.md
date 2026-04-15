---
name: workspace
description: Workspace crate dependency graph, layering rules, key concepts, and architectural patterns
---

# Workspace Architecture

## Crate Dependency Graph

```
core (foundation — no internal dependencies)
  ↑
  ├── workflow (DSL engine + job queue)
  │     └── depends on: core
  │
  ├── license (license verification)
  │     └── depends on: core
  │
  ├── persistence (SQLx repositories)
  │     └── depends on: core, workflow
  │
  ├── services (business logic)
  │     └── depends on: core, persistence, workflow, license
  │
  ├── api (HTTP endpoints)
  │     └── depends on: core, persistence, services, workflow, license
  │
  ├── worker (background tasks)
  │     └── depends on: core, persistence, services, workflow
  │
  └── test-support (dev-only test helpers)
        └── depends on: core, persistence, services, workflow
```

## Layering Rules

- **core** is the foundation — pure domain models, no I/O dependencies
- **persistence** implements repository traits, depends on core + workflow models
- **services** contains business logic, injects repositories via adapters
- **api** is the HTTP boundary — depends on everything, exposes nothing internal
- **worker** runs background jobs — same layer as api but for async processing
- **workflow** is self-contained DSL engine — only depends on core

## Key Concepts

| Concept | Description |
|---------|-------------|
| Entity Definitions | Schema definitions for dynamic entities with field types, validation rules, and UI settings |
| Dynamic Entities | Runtime-created data objects stored in `entities_registry` with JSONB fields |
| Auto-created Views | Each entity type gets `entity_{type}` table and `entity_{type}_view` joining metadata with custom fields |
| Workflows | DSL-based data pipelines with two queues (fetch/process) using Apalis + Redis |

## Binaries

| Binary | Purpose |
|--------|---------|
| `r_data_core` | Main application server (Actix-web) |
| `r_data_core_worker` | Workflow worker (processes jobs from Redis) |
| `r_data_core_maintenance` | Maintenance worker (scheduled tasks) |
| `run_migrations` | SQLx database migrations |
| `clear_cache` | Redis cache management |
| `hash_password` | Password hashing (Argon2) |
| `apply_schema` | Entity schema application |
| `license_tool` | License management CLI |

## Frontend

- **fe/** — Admin frontend (Vue3 + TypeScript + Vite + Vuetify + Pinia), runs in Docker `node` service
