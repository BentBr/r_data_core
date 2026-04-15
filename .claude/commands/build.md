# Build Project

Build the Rust backend or frontend components.

## Rust Backend

### Debug build
```bash
cargo build --workspace
```

### Release build
```bash
cargo build --workspace --release
```

### Check compilation (faster, no artifacts)
```bash
cargo check --workspace
```

### With SQLX offline mode
```bash
SQLX_OFFLINE=true cargo build --workspace
```

## Frontend

### Admin frontend (fe/)
```bash
docker compose exec node npm run build
# or
cd fe && npm run build
```

### Static website
```bash
docker compose exec node-static npm run build
```

## Binaries produced

After building, these binaries are available:
- `r_data_core` - Main API server
- `r_data_core_worker` - Workflow job processor
- `r_data_core_maintenance` - Maintenance worker
- `run_migrations` - Database migrations
- `clear_cache` - Redis cache clearing
- `hash_password` - Password hashing utility
- `apply_schema` - Schema application

## Instructions

1. For quick iteration, use `cargo check`
2. For running locally, use `cargo build`
3. For production/benchmarks, use `cargo build --release`
