# RDataCore

A self-hosted master data management (MDM) platform built with Rust. Connect Shops, PIM, CRM, ERP, and any system. Fast, secure, and flexible.

## Features

- **Dynamic Entity System** - Create custom data structures at runtime through the API
- **Workflow Engine** - DSL-based data pipelines with scheduled and on-demand execution ([DSL Documentation](./docs/DSL.md))
- **API Authentication** - JWT and API key support with role-based access control
- **Import/Export** - CSV, JSON, XML, and third-party API integrations
- **Versioning** - Full version history for entities, definitions, and workflows
- **Self-Hosted** - Your data stays on your infrastructure

## Requirements

| Component | Version |
|-----------|---------|
| Docker | 20.10+ |
| PostgreSQL | 14+ |
| Redis | 7+ |

For development, you'll also need:
- Rust 1.92+ (nightly)
- Node.js 22+ (for admin frontend)

## Quick Start

### Using Docker (Recommended)

1. Clone the repository:
```bash
git clone https://github.com/BentBr/r_data_core.git
cd r_data_core
```

2. Copy (and adjust to your needs) the environment:
```bash
cp .env.example .env
```

3. Start all services:
```bash
docker compose up -d
```

The application will be available at `http://rdatacore.docker` if you setup dinghy DNS routing:
1. Install Dinghy (just another docker container) - for MAC OS:
    ```bash
   docker run -d --restart=always \
   -v /var/run/docker.sock:/tmp/docker.sock:ro \
   -v ~/.dinghy/certs:/etc/nginx/certs \
   -p 80:80 -p 443:443 -p 19322:19322/udp \
   -e CONTAINER_NAME=http-proxy \
   --name http-proxy \
   codekitchen/dinghy-http-proxy
   ```
2. Setup routing
    ```bash
    sudo mkdir -pv /etc/resolver
    sudo bash -c 'echo "nameserver 127.0.0.1" > /etc/resolver/docker'
    sudo bash -c 'echo "port 19322" >> /etc/resolver/docker'
    ```

If you are not on macOS, you should create an `compose.override.yaml` and re-assign ports to for the web service to your localhosts.

### Using Pre-built Docker Images

Pull the latest images from GitHub Container Registry:

```bash
# Main application
docker pull ghcr.io/bentbr/r-data-core:latest

# Workflow worker
docker pull ghcr.io/bentbr/r-data-core-worker:latest

# Maintenance worker
docker pull ghcr.io/bentbr/r-data-core-maintenance:latest
```

## Configuration

### Required Environment Variables

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection string |
| `JWT_SECRET` | Secret key for JWT token signing |
| `REDIS_URL` | Redis connection URL |
| `LICENSE_KEY` | JWT-based license key for this instance |

### Optional Environment Variables

| Variable | Default     | Description                           |
|----------|-------------|---------------------------------------|
| `APP_ENV` | development | Application environment               |
| `API_HOST` | 0.0.0.0     | Server host address                   |
| `API_PORT` | 8888        | Server port                           |
| `JWT_EXPIRATION` | 86400       | JWT token expiration (seconds)        |
| `API_ENABLE_DOCS` | true        | Enable Swagger API documentation      |
| `CORS_ORIGINS` | *           | Allowed CORS origins                  |
| `CACHE_ENABLED` | true        | Enable caching                        |
| `CACHE_TTL` | 300         | Default cache TTL (seconds)           |
| `CHECK_DEFAULT_ADMIN_PASSWORD` | true        | Defines if the warning in FE is shown |
| `LICENSE_PRIVATE_KEY` | -           | RSA private key for creating license keys (used by `license_tool` binary) |

### Maintenance Worker Environment Variables

| Variable | Description |
|----------|-------------|
| `VERSION_PURGER_CRON` | Cron expression for version purger task |
| `REFRESH_TOKEN_CLEANUP_CRON` | Cron expression for refresh token cleanup task |
| `MAINTENANCE_DATABASE_URL` | PostgreSQL connection string for maintenance worker |
| `MAINTENANCE_DATABASE_MAX_CONNECTIONS` | Maximum database connections (default: 10) |
| `MAINTENANCE_DATABASE_CONNECTION_TIMEOUT` | Connection timeout in seconds (default: 30) |

See `.env.example` for the complete list of configuration options.

## Architecture

RDataCore consists of three main components:

- **API Server** (`r_data_core`) - Handles HTTP requests, authentication, and entity management
- **Workflow Worker** (`r_data_core_worker`) - Processes workflow jobs from Redis queue
- **Maintenance Worker** (`r_data_core_maintenance`) - Runs scheduled maintenance tasks

### Utility Binaries

The main Docker image includes utility binaries for operations and maintenance:

```bash
# Run database migrations
docker compose exec core /usr/local/bin/run_migrations

# Check migration status
docker compose exec core /usr/local/bin/run_migrations --status

# Clear entire Redis cache
docker compose exec core /usr/local/bin/clear_cache --all

# Clear specific cache by prefix
docker compose exec core /usr/local/bin/clear_cache --prefix "entity_definitions:"

# Preview cache deletion (dry-run)
docker compose exec core /usr/local/bin/clear_cache --prefix "api_keys:" --dry-run

# Hash a password for admin users
docker compose exec core /usr/local/bin/hash_password 'YourSecurePassword'

# Generate RSA keypair for license keys (one-time setup)
openssl genrsa -out license_private.key 2048
openssl rsa -in license_private.key -pubout -out license_public.key

# Create a new license key
docker compose exec core /usr/local/bin/license_tool create \
  --company "Your Company Name" \
  --license-type Enterprise \
  --private-key-path /path/to/license_private.key \
  --expires-days 365

# Verify an existing license key
docker compose exec core /usr/local/bin/license_tool verify \
  --license-key "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..." \
  --public-key-path /path/to/license_public.key

# Check license key against verification API (uses LICENSE_KEY from environment)
docker compose exec core /usr/local/bin/license_tool check
```

| Binary | Description |
|--------|-------------|
| `run_migrations` | Run SQLx database migrations (`--status` to check, `--help` for options) |
| `clear_cache` | Clear Redis cache (`--all` or `--prefix <PREFIX>`, `--dry-run` to preview) |
| `hash_password` | Generate Argon2 password hash with SQL update statement |
| `license_tool` | Create, verify, and check JWT-based license keys (`create`, `verify`, or `check` subcommands) |

### Database Schema

Key tables:
- `entity_definitions` - Schema definitions for dynamic entities
- `entities_registry` - All entity instances with JSONB field storage
- `workflows` - Workflow definitions with DSL configuration
- `workflow_runs` - Workflow execution history
- `admin_users` - Admin user accounts
- `api_keys` - API authentication keys

## API Documentation

Once running, access the API documentation at:

- **Public API**: `http://rdatacore.docker/api/docs/`
- **Admin API**: `http://rdatacore.docker/admin/api/docs/`

### API Endpoints

**Admin API** (requires admin JWT):
- `GET/POST /admin/api/v1/entity-definitions` - Manage entity schemas
- `GET/POST /admin/api/v1/workflows` - Manage workflows
- `GET/POST /admin/api/v1/admin-users` - Manage admin users
- `GET/POST /admin/api/v1/api-keys` - Manage API keys

**Public API** (JWT or API key):
- `GET/POST /api/v1/entities/{type}` - CRUD operations on entities

## Entity System

### Entity Definitions

Define custom data structures with field types, validation rules, and UI settings:

```json
{
  "entity_type": "products",
  "display_name": "Products",
  "fields": [
    {
      "name": "sku",
      "field_type": "String",
      "required": true,
      "unique": true
    },
    {
      "name": "price",
      "field_type": "Float",
      "required": true
    }
  ]
}
```

### Supported Field Types

- **Text**: String, Text, Wysiwyg
- **Numeric**: Integer, Float
- **Boolean**: Boolean
- **Date**: Date, DateTime
- **Complex**: Object, Array, UUID
- **Relations**: ManyToOne, ManyToMany
- **Select**: Select, MultiSelect
- **Assets**: Image, File

## Workflows

Create automated data pipelines using the workflow DSL:

1. **Fetch Stage** - Pull data from external sources (APIs, files, databases)
2. **Transform Stage** - Apply transformations and business logic
3. **Process Stage** - Store, export, or forward processed data

Workflows can be triggered by:
- Cron schedules
- Manual API calls
- Webhook events

## Support

- **Documentation**: [API Docs](https://rdatacore.eu/api/docs/)
- **Issues**: [GitHub Issues](https://github.com/BentBr/r_data_core/issues)
- **Contact**: hello@rdatacore.eu

## Development

For development setup, testing, and contribution guidelines, see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## License

See [Pricing](https://rdatacore.eu/en/pricing) for license information.

- **Free** for developers, educators, and small teams
- **Commercial licenses** available for organizations
