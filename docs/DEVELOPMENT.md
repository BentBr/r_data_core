# Development Guide

This document covers development setup, testing, and contribution guidelines for RDataCore.

## Contributing

### Signed Commits

All commits must be signed. Configure Git to sign your commits:

```bash
# Configure GPG signing
git config --global user.signingkey YOUR_GPG_KEY_ID
git config --global commit.gpgsign true

# Or use SSH signing (Git 2.34+)
git config --global gpg.format ssh
git config --global user.signingkey ~/.ssh/id_ed25519.pub
git config --global commit.gpgsign true
```

Unsigned commits will be rejected by branch protection rules.

### Conventional Commits

This project uses [Conventional Commits](https://www.conventionalcommits.org/) for automated versioning and changelog generation via release-please.

**Commit message format:**
```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Required prefixes:**

| Prefix | Description | Version Bump |
|--------|-------------|--------------|
| `feat:` | New feature | Minor |
| `fix:` | Bug fix | Patch |
| `docs:` | Documentation only | None |
| `style:` | Code style (formatting, semicolons) | None |
| `refactor:` | Code change that neither fixes a bug nor adds a feature | None |
| `perf:` | Performance improvement | Patch |
| `test:` | Adding or correcting tests | None |
| `build:` | Changes to build system or dependencies | None |
| `ci:` | Changes to CI configuration | None |
| `chore:` | Other changes that don't modify src or test files | None |

**Breaking changes:**

Add `!` after the type or include `BREAKING CHANGE:` in the footer:
```
feat!: remove deprecated API endpoints

BREAKING CHANGE: The /api/v1/legacy endpoints have been removed.
```

**Examples:**
```bash
git commit -m "feat(api): add bulk entity import endpoint"
git commit -m "fix(workflow): correct timeout handling in fetch stage"
git commit -m "docs: update API authentication guide"
git commit -m "refactor(persistence): simplify query builder"
```

## Workspace Layout

This repository is organized as a Cargo workspace:

- `crates/api` - Actix Web API (admin/public), middleware
- `crates/core` - Domain models, versioning, permissions, prelude, error types
- `crates/persistence` - SQLx repositories and DB utilities
- `crates/services` - Business logic and data handling
- `crates/workflow` - Workflow engine and DSL
- `crates/worker` - Background workers and maintenance tasks
- `crates/test-support` - Shared test helpers (dev dependency only)

### Frontends

- `fe/` - Admin frontend (Vue3 + TypeScript + Vite + Vuetify + Pinia)
- `static-website/` - Public static website (Vue3 + TypeScript + Vite + Vuetify)

Both frontends run in Docker containers (`node` and `node-static` services).

## Development Setup

### Prerequisites

- Rust 1.92+ (nightly)
- Docker and Docker Compose
- Node.js 20+ (for admin frontend)
- Node.js 22+ (for static website)

### Initial Setup

1. Clone the repository:
```bash
git clone https://github.com/BentBr/r_data_core.git
cd r_data_core
```

2. Start database and Redis:
```bash
docker compose up -d postgres redis
```

3. Set up environment variables:
```bash
cp .env.example .env
```

4. Run database migrations:
```bash
cargo sqlx migrate run
```

5. Start the development server:
```bash
RUST_BACKTRACE=1 RUST_LOG=info cargo run --bin r_data_core
```

## Using RDT (Rusty Dev Tool)

This project uses `rdt` for common development tasks:

```bash
rdt test            # Run all workspace tests
rdt test-unit       # Run unit tests only
rdt test-fe         # Run admin frontend tests (vitest via Docker)
rdt test-fe-static  # Run static website tests (vitest via Docker)
rdt clippy          # Run clippy with strict lints
rdt lint            # Run ESLint + Prettier for admin frontend
rdt lint-static     # Run ESLint + Prettier for static website
```

## Building & Running

### Binaries

```bash
cargo run --bin r_data_core           # Main application server
cargo run --bin r_data_core_worker    # Workflow worker
cargo run --bin r_data_core_maintenance  # Maintenance worker
cargo run --bin hash_password         # Hash passwords for admin users
cargo run --bin clear_cache           # Clear cache (see --help)
cargo run --bin run_migrations        # Run database migrations
cargo run --bin apply_schema          # Apply schema changes
```

### Run with Logging

```bash
RUST_BACKTRACE=1 RUST_LOG=info cargo run --bin r_data_core
```

Use `RUST_LOG=debug` for extensive output.

## Database Operations

### Migrations

```bash
# Run migrations
cargo sqlx migrate run

# Prepare test database
DATABASE_URL="postgres://postgres:postgres@pg-test.rdatacore.docker:5433/rdata_test" cargo sqlx migrate run
```

### Reset Database

```bash
docker compose down -v && docker compose up -d redis postgres && sleep 7 && cargo sqlx migrate run
```

### Update SQLx Queries

After schema changes, regenerate the SQLx query cache:

```bash
cargo sqlx prepare --workspace -- --all-targets
```

## Testing

### Running Tests

```bash
# All tests (use single thread for integration tests)
RUST_LOG=warning cargo test -- --test-threads=1

# Unit tests only (can run concurrently)
cargo test --lib -p r_data_core

# Specific test by name (without test_ prefix)
cargo test my_test_name

# Using rdt
rdt test
```

### Test Structure

- **Unit tests**: Within `src/` files in each crate
- **Integration tests**: `/tests/` directory
- Tests can be run by name without the `test_` prefix

### Prepare Test Database

```bash
DATABASE_URL="postgres://postgres:postgres@pg-test.rdatacore.docker:5433/rdata_test" cargo sqlx migrate run
```

## Code Quality

### Clippy (Strict)

Clippy is enforced strictly across the workspace:

```bash
cargo clippy --workspace --all-targets --all-features -- \
  -D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery
```

### Formatting

```bash
cargo fmt --check --all
```

## SQLx Notes

This project uses SQLx with compile-time query verification. This means:

- A running PostgreSQL database is required during compilation
- The database must have all required tables and schema
- Database migrations must be run before compiling

If you encounter compilation errors about missing tables:
1. Ensure the database is running: `docker compose up -d postgres redis`
2. Run migrations: `cargo sqlx migrate run`

## Environment Variables

### Application (Main Server)

**Mandatory:**
- `DATABASE_URL` - PostgreSQL connection string
- `JWT_SECRET` - Secret key for JWT token signing
- `REDIS_URL` - Redis connection URL

**Optional:**
- `APP_ENV` - Application environment (default: "development")
- `API_HOST` - Server host address (default: "0.0.0.0")
- `API_PORT` - Server port (default: 8888)
- `API_USE_TLS` - Enable SSL/TLS (default: false)
- `JWT_EXPIRATION` - JWT token expiration in seconds (default: 86400)
- `API_ENABLE_DOCS` - Enable API documentation (default: true)
- `CORS_ORIGINS` - Comma-separated list of allowed CORS origins (default: "*")
- `DATABASE_MAX_CONNECTIONS` - Maximum database connections (default: 10)
- `DATABASE_CONNECTION_TIMEOUT` - Connection timeout in seconds (default: 30)
- `LOG_LEVEL` - Logging level: info/debug/error (default: "info")
- `LOG_FILE` - Optional log file path
- `CACHE_ENABLED` - Enable caching (default: true)
- `CACHE_TTL` - Default cache TTL in seconds (default: 300)
- `CACHE_MAX_SIZE` - Maximum cache size in items (default: 10000)
- `CACHE_ENTITY_DEFINITION_TTL` - Entity definition cache TTL, 0 = infinite (default: 0)
- `CACHE_API_KEY_TTL` - API key cache TTL in seconds (default: 600)
- `QUEUE_FETCH_KEY` - Redis key for fetch jobs queue (default: "queue:workflows:fetch")
- `QUEUE_PROCESS_KEY` - Redis key for process jobs queue (default: "queue:workflows:process")

### Workflow Worker

**Mandatory:**
- `WORKER_DATABASE_URL` - PostgreSQL connection string for worker
- `REDIS_URL` - Redis connection URL
- `JOB_QUEUE_UPDATE_INTERVAL` - Interval to reconcile scheduled jobs (must be > 0)

**Optional:**
- `WORKER_DATABASE_MAX_CONNECTIONS` - Maximum database connections (default: 10)
- `WORKFLOW_WORKER_THREADS` - Number of worker threads (default: 4)
- `WORKFLOW_DEFAULT_TIMEOUT` - Default workflow timeout in seconds (default: 300)
- `WORKFLOW_MAX_CONCURRENT` - Maximum concurrent workflows (default: 10)

### Maintenance Worker

**Mandatory:**
- `MAINTENANCE_DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection URL

**Optional:**
- `MAINTENANCE_CRON` - Cron expression for scheduler (default: "*/5 * * * *")
- `MAINTENANCE_DATABASE_MAX_CONNECTIONS` - Maximum database connections (default: 10)

## Error Handling

This project uses a consistent error handling approach across crates:

### Core Error Type

All application code uses `r_data_core_core::error::Result<T>` and `r_data_core_core::error::Error` for error handling. The error type is defined in `crates/core/src/error.rs` and provides:

- **Structured error variants**: Database, IO, Auth, Config, API, Cache, Serialization, Validation, etc.
- **Automatic conversions**: From common error types (sqlx::Error, std::io::Error, serde_json::Error, etc.)
- **Type alias**: `r_data_core_core::error::Result<T>` for convenience

**Note**: The `anyhow` crate is **not** used in library code. It is only available as a dev-dependency for integration tests in the `tests/` directory.

### Workflow Crate

The `crates/workflow` crate uses `r_data_core_core::error::Result<T>`:

- **No `anyhow` dependency**: The workflow crate has been migrated away from `anyhow`
- **All functions return**: `r_data_core_core::error::Result<T>` or `r_data_core_core::error::Result<()>`
- **Error conversion**: External errors (CSV parsing, HTTP requests, etc.) are converted to `r_data_core_core::error::Error` using appropriate variants
- **Examples**:
  - `validate_mapping()` returns `r_data_core_core::error::Result<()>`
  - `DslProgram::from_config()` returns `r_data_core_core::error::Result<Self>`
  - All adapter traits (DataSource, FormatHandler, DataDestination) use `r_data_core_core::error::Result<T>`

### Worker Crate

The `crates/worker` crate uses `r_data_core_core::error::Result<T>`:

- **No `anyhow` dependency**: The worker crate has been migrated away from `anyhow`
- **All functions return**: `r_data_core_core::error::Result<T>` or `r_data_core_core::error::Result<()>`
- **Error conversion**: External errors (JobScheduler errors, database pool initialization, etc.) are converted to `r_data_core_core::error::Error` using appropriate variants
- **Entry points**: Both `main.rs` and `maintenance.rs` use `r_data_core_core::error::Result<()>`
- **Examples**:
  - `main()` returns `r_data_core_core::error::Result<()>`
  - `schedule_job` closure returns `r_data_core_core::error::Result<Uuid>`

### Error Handling Best Practices

1. **Use `r_data_core_core::error::Result<T>`** in library code (workflow, services, persistence, api, core)
2. **Use appropriate error variants**: `Error::Validation()` for validation errors, `Error::Config()` for configuration issues, etc.
3. **Convert external errors**: Use `.map_err()` to convert third-party errors to `r_data_core_core::error::Error`
4. **Preserve context**: Include relevant information in error messages (step indices, field names, etc.)
5. **Do not use `anyhow`** in library code - it is only available in integration tests (`tests/` directory)

## Architecture Details

### Queue System (Apalis)

The system uses Apalis with Redis for workflow job queuing:

- **Redis Lists**: Uses Redis Lists (RPUSH/BLPOP) for queue operations
- **Two Queues**:
  - `fetch` queue: Jobs for fetching and staging data from external sources
  - `process` queue: Jobs for processing staged items
- **Blocking Operations**: Workers use `BLPOP` to block until jobs are available

### Workflow Execution Flow

1. Scheduled workflows are scanned by the worker
2. Jobs are enqueued to Redis `fetch` queue
3. Worker pops jobs, creates workflow runs, and processes them
4. Data is staged in `workflow_raw_items` table
5. Staged items are processed according to workflow DSL
6. Workflow runs are marked as success/failure

### Maintenance Tasks

The maintenance worker runs scheduled tasks (`MAINTENANCE_CRON`):

- **Entity Version Pruning**: Removes old entity versions based on:
  - **By Age**: Versions older than `max_age_days` setting
  - **By Count**: Keeps only latest N versions (`max_versions` setting)

## Entity System Details

### Auto-Created Views

For each entity type, the system automatically creates:

- A table `entity_{entity_type}` for storing entity-specific field data
- A view `entity_{entity_type}_view` that joins `entities_registry` with the entity table
- INSTEAD OF triggers for transparent INSERT/UPDATE operations

### Example JSON Files

See [.example_files/json_examples](.example_files/json_examples) for entity definition examples.

## API Routes

- `/api/docs/` - Public API Swagger documentation
- `/admin/api/docs/` - Admin API Swagger documentation

## Todos / Roadmap

See the main repository issues for current development priorities.

## Cache Management

```bash
cargo run --bin clear_cache -- --help
```

Lists available cache clearing options.

## MSRV

Minimum Supported Rust Version: **1.92.0**
