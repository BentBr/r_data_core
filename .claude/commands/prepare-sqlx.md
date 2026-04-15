# Prepare SQLx Queries

Prepare SQLx query cache for offline compilation after schema changes.

## Command

```bash
cargo sqlx prepare --workspace -- --all-targets
```

## When to run

Run this command after:
1. Adding or modifying SQL queries in the code
2. Changing database migrations
3. Before committing if `.sqlx/` files changed

## What it does

- Connects to the database and validates all SQL queries
- Generates `.sqlx/` cache files for compile-time verification
- Enables `SQLX_OFFLINE=true` builds without database connection

## Prerequisites

- PostgreSQL must be running with current migrations applied
- Database connection configured in `.env` or `DATABASE_URL`

## Troubleshooting

If preparation fails:
1. Ensure database is running: `docker compose up -d postgres`
2. Run migrations: `cargo sqlx migrate run`
3. Check `DATABASE_URL` is correct
4. Verify SQL syntax in the failing query
