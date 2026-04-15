# Docker Commands

Manage Docker services for development.

## Services

From `compose.yaml`:
- `postgres` - Main PostgreSQL database (port 5432)
- `pg-test` - Test PostgreSQL database (port 5433)
- `redis` - Redis cache (port 6379)
- `core` - Main API server
- `worker` - Workflow worker
- `maintenance` - Maintenance worker
- `node` - Admin frontend (Vue)
- `node-static` - Static website

## Common commands

### Start services
```bash
# All services
docker compose up -d

# Just databases
docker compose up -d postgres redis pg-test

# Backend services
docker compose up -d core worker maintenance

# Frontend
docker compose up -d node node-static
```

### Stop services
```bash
# Stop all
docker compose down

# Stop and remove volumes (full reset)
docker compose down -v
```

### Logs
```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f core
docker compose logs -f worker
```

### Execute commands in containers
```bash
# Run migrations
docker compose exec core /usr/local/bin/run_migrations --status

# Clear cache
docker compose exec core /usr/local/bin/clear_cache --all

# Hash password
docker compose exec core /usr/local/bin/hash_password 'newpassword'

# Frontend npm commands
docker compose exec node npm run lint
docker compose exec node npm run build
```

### Rebuild images
```bash
docker compose build
docker compose build --no-cache core
```

## Hostnames

Services are available at:
- `rdatacore.docker` - Main API
- `pg.rdatacore.docker:5432` - Main database
- `pg-test.rdatacore.docker:5433` - Test database
- `redis.rdatacore.docker:6379` - Redis
