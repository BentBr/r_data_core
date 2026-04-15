# Reset Database

Reset the development database to a clean state.

## Full reset command

```bash
docker compose down -v && docker compose up -d redis postgres && sleep 7 && cargo sqlx migrate run
```

This will:
1. Stop and remove all containers and volumes
2. Start fresh PostgreSQL and Redis containers
3. Wait for services to be ready
4. Run all migrations

## Partial reset (keep containers)

```bash
# Just drop and recreate the database
docker compose exec postgres psql -U postgres -c "DROP DATABASE IF EXISTS rdata;"
docker compose exec postgres psql -U postgres -c "CREATE DATABASE rdata;"
cargo sqlx migrate run
```

## Clear cache only

```bash
./target/release/clear_cache --all
# or in Docker:
docker compose exec core /usr/local/bin/clear_cache --all
```

## Instructions

1. Use full reset when you need a completely clean slate
2. Use partial reset to keep Docker containers running
3. After reset, you may need to create a default admin user

## Warning

This will DELETE all data in the development database. Use with caution.
