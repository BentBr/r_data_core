# Run Application

Start the application services locally.

## Main services

### API Server
```bash
RUST_BACKTRACE=1 RUST_LOG=info cargo run --bin r_data_core
```

### Workflow Worker
```bash
RUST_BACKTRACE=1 RUST_LOG=info cargo run --bin r_data_core_worker
```

### Maintenance Worker
```bash
RUST_BACKTRACE=1 RUST_LOG=info cargo run --bin r_data_core_maintenance
```

## Using Docker Compose

```bash
# Start all services
docker compose up -d

# Start specific services
docker compose up -d postgres redis
docker compose up -d core worker
```

## Environment variables

Key environment variables:
- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string
- `RUST_LOG` - Log level (error, warn, info, debug, trace)
- `RUST_BACKTRACE` - Enable backtraces (1 or full)

## Access points

- API: `http://rdatacore.docker/api/`
- Admin API: `http://rdatacore.docker/admin/api/`
- Swagger (Public): `http://rdatacore.docker/api/docs/`
- Swagger (Admin): `http://rdatacore.docker/admin/api/docs/`
