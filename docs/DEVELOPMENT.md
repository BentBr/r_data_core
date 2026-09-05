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
RUST_BACKTRACE=1 RUST_LOG=info cargo run --bin r-data-core
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
cargo run --bin r-data-core           # Main application server
cargo run --bin r-data-core-worker    # Workflow worker
cargo run --bin r-data-core-maintenance  # Maintenance worker
cargo run --bin hash-password         # Hash passwords for admin users
cargo run --bin clear-cache           # Clear cache (see --help)
cargo run --bin run-migrations        # Run database migrations
cargo run --bin apply-schema          # Apply schema changes
```

### Run with Logging

```bash
RUST_BACKTRACE=1 RUST_LOG=info cargo run --bin r-data-core
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

### CI Guards

Beyond clippy and rustfmt, CI enforces four structural rules. Run them locally
before pushing — they are cheap and fail fast:

```bash
./scripts/check-sql-boundary.sh   # SQL only inside crates/persistence
./scripts/check-file-length.sh    # 300-line soft cap, 500-line hard cap
cargo test --test architecture      # crate layering (cargo-metadata based)
cargo +1.96.0 check --workspace    # MSRV
```

### No Panics in Production Code

Every production crate root denies the panicking escape hatches:

```rust
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]
```

Where an unwrap is genuinely infallible, add a narrowly scoped
`#[allow(...)]` **with a comment saying why**. Tests are exempt.

## Security & Operations

### Client IP and Rate Limiting

The unauthenticated auth endpoints (`/auth/login`, `/auth/register`) are limited
per client IP. Behind a reverse proxy every request carries the proxy's peer
address, so the limit would apply to everyone at once unless the deployment
tells the server which proxies to trust:

1. Set `TRUSTED_PROXIES` to the proxy's address or CIDR block.
2. Make sure that proxy sets `X-Forwarded-For`.

`X-Forwarded-For` is only read when the immediate peer is in `TRUSTED_PROXIES`;
otherwise the peer address is used and a forged header is ignored. Requests with
no peer address share one `unknown` bucket.

The counter is an atomic Redis `INCR` with an expiry set once per window, so
parallel requests cannot undercount and the window does not slide forward on
every hit.

### Account Lockout and Unlocking

`LOGIN_MAX_FAILED_ATTEMPTS` consecutive bad passwords lock an account for
`LOGIN_LOCKOUT_DURATION_SECS`. The lock lifts itself on the next login once the
expiry has passed — an attacker cannot park an admin account in the locked state
indefinitely. Setting the duration to `0` makes locks permanent, which then
requires one of the recovery paths below.

Three ways back in:

| Path | Who | Notes |
|------|-----|-------|
| Admin UI / API | An operator with `Users:Update` | `PUT /admin/api/v1/users/{uuid}` with `{"status": "active"}`; the Users tab shows a lock icon on locked accounts |
| Password reset | The user | The reset flow clears the lockout; needs mail configured |
| `user-actions` CLI | An operator with DB access | Last resort when nobody can reach the UI |

```bash
cargo run --bin user-actions -- --username <name> --action unlock
# also: lock | activate | deactivate | password-reset --password <new>
```

An operator lock (CLI or `{"status": "locked"}`) carries no expiry and never
lifts itself.

### Environment Hardening

`AppConfig::is_hardened()` gates the environment-sensitive policy. It fails
closed: only `development`, `dev`, `local` and `test` are treated as relaxed, so
staging, preprod and an unset `APP_ENV` are hardened like production.

In a hardened environment:

- `CORS_ORIGINS` must be set to explicit, non-wildcard origins or the server
  refuses to start.
- The SSRF guard in the workflow HTTP adapters blocks private, loopback,
  link-local and IPv4-mapped addresses, re-validating every DNS resolution and
  every redirect hop. `SSRF_ALLOWED_HOSTS` is the escape hatch.

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

**Security (see [Security & Operations](#security--operations)):**
- `TRUSTED_PROXIES` - Comma-separated IPs/CIDRs of reverse proxies whose `X-Forwarded-For` may be believed (default: empty)
- `LOGIN_MAX_FAILED_ATTEMPTS` - Failed passwords before an account is locked (default: 5)
- `LOGIN_LOCKOUT_DURATION_SECS` - How long a lock lasts before it expires; 0 = until an operator unlocks (default: 900)
- `LOGIN_RATE_LIMIT_MAX_ATTEMPTS` - Failed logins per client IP per window (default: 10)
- `LOGIN_RATE_LIMIT_WINDOW_SECS` - Rate-limit window in seconds (default: 900)
- `SSRF_ALLOWED_HOSTS` - Comma-separated hosts the workflow HTTP adapters may reach even when they resolve to a blocked address (default: empty)
- `PASSWORD_RESET_THROTTLE_SECONDS` - Minimum seconds between password-reset requests for one account (default: 60)

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
cargo run --bin clear-cache -- --help
```

Lists available cache clearing options.

## MSRV

Minimum Supported Rust Version: **1.92.0**
