---
name: test-support-crate
description: Shared test helpers, fixtures, database setup/teardown in crates/test-support/
---

# Test Support Crate (`r_data_core_test_support`)

**Path**: `crates/test-support/`
**Role**: Shared testing utilities and fixtures. Dev dependency only.
**Depends on**: core, persistence, services, workflow

## Key Modules

### `database.rs` — Database Setup/Teardown

| Function | Purpose |
|----------|---------|
| `setup_test_db()` | Create isolated test database schema |
| `teardown_test_schema()` | Clean up test schema |
| `clear_test_db()` | Clear all test data |
| `fast_clear_test_db()` | Fast data clearing (truncate) |
| `cleanup_orphaned_test_schemas()` | Remove leaked test schemas |
| `unique_entity_type()` | Generate unique entity type name |
| `random_string()` | Generate random string for tests |

### `entities.rs` — Test Fixtures

| Function | Purpose |
|----------|---------|
| `create_test_admin_user()` | Create admin user fixture |
| `create_test_api_key()` | Create API key fixture |
| `create_test_entity()` | Create dynamic entity fixture |
| `create_test_entity_definition()` | Create entity definition fixture |
| `create_entity_definition_from_json()` | Create definition from JSON |

### `queue.rs` — Queue Utilities

| Function | Purpose |
|----------|---------|
| `test_queue_client()` / `test_queue_client_async()` | Create test queue client |
| `make_workflow_service()` | Create workflow service for tests |

### `consumer_loop.rs` — Consumer Simulation

| Item | Purpose |
|------|---------|
| `spawn_test_consumer_loop()` | Run async consumer for tests |
| `create_test_queue()` | Create isolated test queue |
| `ConsumerLoopConfig` | Consumer configuration |
| `ConsumerLoopHandle` | Handle to control consumer lifecycle |

## Patterns

- Each test gets an isolated database schema
- Fixtures create minimal valid objects
- Cleanup happens automatically in test teardown
- `future_not_send` allows needed in test files using `setup_test_app()`
