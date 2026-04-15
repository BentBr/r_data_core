---
name: database
description: SQLx database setup, migrations, compile-time verification, and test database configuration
---

# Database

## Overview

- **ORM**: SQLx with compile-time query verification
- **Database**: PostgreSQL 14+
- **Migrations**: `/migrations/` directory
- **Test DB**: `pg-test.rdatacore.docker:5433`

## Compile-time Query Verification

SQLx verifies queries at compile time against a running Postgres instance. After schema changes:

```bash
cargo sqlx prepare --workspace -- --all-targets
```

## Migrations

```bash
# Run migrations
cargo sqlx migrate run

# Check migration status
cargo run --bin run_migrations -- --status

# In Docker
docker compose exec core /usr/local/bin/run_migrations --status
```

## Test Database

```bash
# Prepare test database
DATABASE_URL="postgres://postgres:postgres@pg-test.rdatacore.docker:5433/rdata_test" cargo sqlx migrate run
```

Test schemas are created/dropped during test runs automatically.

## Reset Database

```bash
docker compose down -v && docker compose up -d redis postgres && sleep 7 && cargo sqlx migrate run
```

## Dynamic Entity Schema

Each entity type gets auto-created database objects:
- `entity_{type}` table — stores the custom JSONB fields
- `entity_{type}_view` — joins `entities_registry` metadata with custom fields

Schema changes are applied via:
```bash
cargo run --bin apply_schema
```

Or via the admin API: `POST /admin/api/v1/entity-definitions/apply-schema`
