# Database Migrations

Manage SQLx database migrations for this project.

## Commands

### Run migrations
```bash
cargo sqlx migrate run
```

### Check migration status
```bash
./target/release/run_migrations --status
# or in Docker:
docker compose exec core /usr/local/bin/run_migrations --status
```

### Create new migration
```bash
cargo sqlx migrate add <migration_name>
```

### Prepare test database
```bash
DATABASE_URL="postgres://postgres:postgres@pg-test.rdatacore.docker:5433/rdata_test" cargo sqlx migrate run
```

## Instructions

1. After creating/modifying migrations, run `cargo sqlx prepare --workspace -- --all-targets`
2. Migrations are in `/migrations/` directory
3. PostgreSQL 14+ is required
4. Test database is separate from development database

## Database connections

- Development: `pg.rdatacore.docker:5432`
- Test: `pg-test.rdatacore.docker:5433`
