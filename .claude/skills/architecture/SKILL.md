---
name: architecture
description: r_data_core system view — workspace crate dependency graph, layering rules, key concepts, and architectural patterns. Read by backend agents and conflict-resolver for boundary decisions.
color: purple
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
  ├── oidc-http (OIDC discovery + JWKS fetching)
  │     └── depends on: core
  │     └── exists so the MCP server can reach it without linking
  │        services/persistence — it is an HTTP client of RDataCore
  │
  ├── persistence (SQLx repositories)
  │     └── depends on: core, workflow
  │
  ├── services (business logic)
  │     └── depends on: core, persistence, workflow, license, oidc-http
  │
  ├── api (HTTP endpoints)
  │     └── depends on: core, persistence, services, workflow, license
  │
  ├── worker (background tasks)
  │     └── depends on: core, persistence, services, workflow
  │
  ├── mcp (Model Context Protocol server)
  │     └── depends on: core, workflow, oidc-http
  │     └── deliberately NOT services/persistence: it reaches RDataCore over
  │        HTTP like any other client, and must not link its database code
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
