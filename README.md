# R Data Core

## Workspace Layout

This repository is organized as a Cargo workspace:

- `crates/core` — domain models, versioning, permissions, prelude
- `crates/persistence` — SQLx repositories and DB utilities
- `crates/api` — Actix Web API (admin/public), middleware
- `crates/worker` — background workers
- `crates/test-support` — shared test helpers

Clippy is enforced strictly across the workspace:

```bash
cargo clippy --workspace --all-targets --all-features -- \
  -D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery
```

A robust backend for flexible data management with dynamic entity system, workflow management, API authentication, Redis caching, PostgreSQL database support, and migration system.

## Features

- Dynamic entity system for flexible data modeling
- Entity definitions with customizable fields
- Entity registry with field validation
- API authentication (JWT and API key)
- Redis caching
- PostgreSQL database support
- Migration system
- Workflow engine with DSL-based definitions and job queue
- Versioning system for entities, entity definitions, and workflows
- Workflow worker for processing scheduled and queued workflows
- Maintenance worker for automated system maintenance tasks
- API documentation at `/api/docs`

## Database Schema

The project uses a dynamic entity model with the following key tables:

- `entity_definitions`: Defines entity types with their field definitions
- `entity_definition_versions`: Tracks changes to entity definitions for versioning
- `entities_registry`: Stores all entities with their field data in a JSONB column
- `entities_versions`: Tracks changes to entities for versioning
- `workflows`: Workflow definitions
- `workflow_versions`: Tracks changes to workflows for versioning
- `workflow_runs`: Workflow execution runs
- `admin_users`: Admin user accounts
- `api_keys`: API keys for authentication
- `permission_schemes`: Permission definitions

## Requirements

- Docker and Docker Compose
- Rust 1.75+ (for development only)
- PostgreSQL 14+
- Redis 7+ (optional, for caching)

## Quick Start

1. Clone the repository:

```bash
git clone https://github.com/yourusername/r-data-core.git
cd r-data-core
```

2. Start the services using Docker Compose:

```bash
docker compose up -d postgres redis
```

3. Run database migrations:

```bash
cargo sqlx migrate run
```

4. Start the application:

```bash
cargo run
```

The application will be available at `http://rdatacore.docker:8888`.

run via docker:

```bash
docker compose up -d
```

The application will be available at `http://rdatacore.docker:80`.

## Development Setup

1. Clone the repository:

```bash
git clone https://github.com/yourusername/r-data-core.git
cd r-data-core
```

2. Start the database and Redis:

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
cargo run
```

Run our application

```bash
# Run the application with logging and backtrace enabled (info is defined in .env, debug for extensive output)
RUST_BACKTRACE=1 RUST_LOG=info cargo run --bin r_data_core
```

renew database:

```bash
docker compose down -v && docker compose up -d redis postgres && sleep 7 && cargo sqlx migrate run
```

update sqlx:
```bash
cargo sqlx prepare --workspace
```

testing:
```bash
# Only r_data_core unit tests
# Unit tests can run concurrently
cargo test --lib -p r_data_core

# All tests, including integration tests in tests/ directory
# We have to set the threads to 1 in order to not have tests running concurrently doing the same things (like altering tables)
RUST_LOG=warning cargo test -- --test-threads=1

# If you did not yet prepare the test database before, run:
DATABASE_URL="postgres://postgres:postgres@pg-test.rdatacore.docker:5433/rdata_test" cargo sqlx migrate run
```

Hint:

- Unit tests are within the `src` directory in each file
- Integration tests are located in the `tests` directory
- Tests can be run by their name without the `test_` prefix

Binaries

- `r_data_core` - The main application binary
- `run_migrations` - The binary for running database migrations (or use `cargo sqlx migrate`)
- `hash_password` - A utility for hashing passwords (for admin users)
- `worker` - Workflow worker that processes workflow jobs from Redis queue
- `maintenance` - Maintenance worker for scheduled tasks (e.g., version pruning)

### Important Note about SQLx

This project uses SQLx, which performs compile-time query verification. This means:

- A running PostgreSQL database is required during compilation
- The database must have all required tables and schema
- Database migrations must be run before compiling the project

If you encounter compilation errors about missing tables, ensure that:

1. The database is running (`docker compose up -d postgres redis`)
2. Migrations have been applied (`cargo sqlx migrate run`)

## API Documentation

Once the server is running, you can access the API documentation at:

```
http://rdatacore.docker/api/docs
```

### Available APIs

todo: see the api docs

## Configuration

The application can be configured using environment variables. See `.env.example` for available options.

### Environment Variables

#### Application (Main Server) - Mandatory
- `DATABASE_URL` - PostgreSQL connection string (required)
- `JWT_SECRET` - Secret key for JWT token signing (required)
- `REDIS_URL` - Redis connection URL for queue and cache (required)

#### Application (Main Server) - Optional
- `APP_ENV` - Application environment (default: "development")
- `API_HOST` - Server host address (default: "0.0.0.0")
- `API_PORT` - Server port (default: 8888)
- `API_USE_TLS` - Enable SSL/TLS (default: false)
- `JWT_EXPIRATION` - JWT token expiration in seconds (default: 86400)
- `API_ENABLE_DOCS` - Enable API documentation (default: true)
- `CORS_ORIGINS` - Comma-separated list of allowed CORS origins (default: "*")
- `DATABASE_MAX_CONNECTIONS` - Maximum database connections in pool (default: 10)
- `DATABASE_CONNECTION_TIMEOUT` - Connection timeout in seconds (default: 30)
- `LOG_LEVEL` - Logging level: info/debug/error (default: "info")
- `LOG_FILE` - Optional log file path
- `CACHE_ENABLED` - Enable caching (default: true)
- `CACHE_TTL` - Default cache TTL in seconds (default: 300)
- `CACHE_MAX_SIZE` - Maximum cache size in items (default: 10000)
- `CACHE_ENTITY_DEFINITION_TTL` - Entity definition cache TTL in seconds, 0 = infinite (default: 0)
- `CACHE_API_KEY_TTL` - API key cache TTL in seconds (default: 600)
- `QUEUE_FETCH_KEY` - Redis key for fetch jobs queue (default: "queue:workflows:fetch")
- `QUEUE_PROCESS_KEY` - Redis key for process jobs queue (default: "queue:workflows:process")

#### Workflow Worker - Mandatory
- `WORKER_DATABASE_URL` - PostgreSQL connection string for worker (required)
- `REDIS_URL` - Redis connection URL for queue (required)
- `JOB_QUEUE_UPDATE_INTERVAL` - Interval in seconds to reconcile scheduled jobs with DB (required, must be > 0)

#### Workflow Worker - Optional
- `WORKER_DATABASE_MAX_CONNECTIONS` - Maximum database connections (default: 10)
- `DATABASE_CONNECTION_TIMEOUT` - Connection timeout in seconds (default: 30)
- `WORKFLOW_WORKER_THREADS` - Number of worker threads (default: 4)
- `WORKFLOW_DEFAULT_TIMEOUT` - Default workflow timeout in seconds (default: 300)
- `WORKFLOW_MAX_CONCURRENT` - Maximum concurrent workflows (default: 10)
- `QUEUE_FETCH_KEY` - Redis key for fetch jobs queue (default: "queue:workflows:fetch")
- `QUEUE_PROCESS_KEY` - Redis key for process jobs queue (default: "queue:workflows:process")
- `CACHE_ENABLED` - Enable caching (default: true)
- `CACHE_TTL` - Default cache TTL in seconds (default: 300)
- `CACHE_ENTITY_DEFINITION_TTL` - Entity definition cache TTL (default: 0 = infinite)
- `CACHE_API_KEY_TTL` - API key cache TTL in seconds (default: 600)

#### Maintenance Worker - Mandatory
- `MAINTENANCE_DATABASE_URL` - PostgreSQL connection string for maintenance worker (required)
- `REDIS_URL` - Redis connection URL for cache (required)

#### Maintenance Worker - Optional
- `MAINTENANCE_CRON` - Cron expression for maintenance scheduler (default: "*/5 * * * *")
- `MAINTENANCE_DATABASE_MAX_CONNECTIONS` - Maximum database connections (default: 10)
- `DATABASE_CONNECTION_TIMEOUT` - Connection timeout in seconds (default: 30)
- `CACHE_ENABLED` - Enable caching (default: true)
- `CACHE_TTL` - Default cache TTL in seconds (default: 300)

## Cache Configuration
@todo: update the cache config and text here

` cargo run --bin clear_cache -- --help` for a list of available cache clear options.
The application supports both in-memory caching and Redis caching. By default, Redis caching is enabled when the `REDIS_URL` environment variable is set.

## Entity System

The R Data Core provides a flexible system for defining and working with dynamic entities. This allows you to create custom data structures at runtime through the API rather than needing to modify the application code.

### Key Concepts

#### Entity Definitions

An **Entity Definition** is a schema that defines the structure of an entity type. It includes:

- A unique identifier
- An entity type name (which becomes the table name)
- A set of field definitions
- Metadata about the entity type (description, display name, etc.)
- Schema information for database representation

When an entity definition is created or updated, the system automatically generates the necessary database tables and columns to store entities of that type.

**Auto-Created Views**: For each entity type, the system automatically creates:
- A table `entity_{entity_type}` for storing entity-specific field data
- A view `entity_{entity_type}_view` that joins the `entities_registry` table (metadata) with the entity-specific table
- The view provides a unified interface for querying entities with both registry metadata (uuid, created_at, updated_at, etc.) and custom fields
- INSTEAD OF triggers on the view handle INSERT and UPDATE operations transparently
- Views are automatically updated when entity definitions change

You can find example JSON files in [example files](.example_files/json_examples)

#### Fields

Each **Field Definition** within an entity definition specifies:

- Field name and display name
- Data type (String, Integer, Boolean, DateTime, etc.)
- Validation rules (required, min/max length, patterns, etc.)
- UI settings for rendering the field in client applications
- Database storage settings (indexed, filterable, etc.)

#### Dynamic Entities

A **Dynamic Entity** is an actual data object that follows the structure defined by a entity definition. Entities can be created, updated, queried, and deleted through the API. Each entity instance includes:

- Standard system fields (UUID, creation timestamp, etc.)
- The custom fields defined in its entity definition
- Metadata and relationship information

### Supported Field Types

The system supports a rich set of field types:

- **Text Types**: String, Text, Wysiwyg (rich text)
- **Numeric Types**: Integer, Float
- **Boolean Type**: Boolean (true/false)
- **Date Types**: Date, DateTime
- **Complex Data Types**: Object, Array, UUID
- **Relation Types**: ManyToOne, ManyToMany
- **Select Types**: Select (single), MultiSelect (multiple)
- **Asset Types**: Image, File

### API Endpoints

All entity definition endpoints are secured with JWT authentication and require admin privileges:

- `GET /admin/api/v1/entity-definitions` - List all entity definitions
- `GET /admin/api/v1/entity-definitions/{uuid}` - Get a specific entity definition
- `POST /admin/api/v1/entity-definitions` - Create a new entity definition
- `PUT /admin/api/v1/entity-definitions/{uuid}` - Update an existing entity definition
- `DELETE /admin/api/v1/entity-definitions/{uuid}` - Delete an entity definition
- `GET /admin/api/v1/entity-definitions/{uuid}/versions` - List versions of an entity definition
- `GET /admin/api/v1/entity-definitions/{uuid}/versions/{version_number}` - Get a specific version

Entity data can be manipulated through the public API endpoints (documentation pending).

### Workers

The system includes two background workers:

- **Workflow Worker** (`worker`): Processes workflow jobs from a Redis queue, handles scheduled workflows based on cron expressions, and processes staged items for data import/export/transformation operations.

- **Maintenance Worker** (`maintenance`): Runs scheduled maintenance tasks such as pruning old entity versions based on configured retention policies (max age and max versions per entity).

### Queue System (Apalis)

The system uses **Apalis** with Redis for workflow job queuing:

#### How It Works
- **Redis Lists**: Uses Redis Lists (RPUSH/BLPOP) for queue operations
- **Two Queues**:
  - `fetch` queue: Jobs for fetching and staging data from external sources
  - `process` queue: Jobs for processing staged items (transformations, imports, exports)
- **Blocking Operations**: Workers use `BLPOP` to block until jobs are available, ensuring efficient resource usage
- **Job Serialization**: Jobs are serialized as JSON and stored in Redis

#### Workflow Execution Flow
1. **Scheduled Workflows**: Workflow worker scans for enabled workflows with cron schedules
2. **Manual Triggers**: Workflows can be triggered via API
3. **Job Enqueueing**: Fetch jobs are enqueued to Redis `fetch` queue
4. **Worker Processing**: Worker pops jobs from queue, creates workflow runs, and processes them
5. **Staging**: Data is fetched from sources and staged in `workflow_raw_items` table
6. **Processing**: Staged items are processed according to workflow DSL configuration
7. **Completion**: Workflow runs are marked as success/failure with statistics

### Maintenance Tasks

The maintenance worker runs scheduled tasks based on cron expression (`MAINTENANCE_CRON`):

- **Entity Version Pruning**: 
  - Prunes old entity versions based on system settings
  - Two pruning strategies:
    - **By Age**: Removes versions older than configured days (`max_age_days` setting)
    - **By Count**: Keeps only the latest N versions per entity (`max_versions` setting)
  - Pruning respects entity versioning settings (can be disabled globally via system settings)
  - Settings are configurable via system settings API (`/admin/api/v1/system/settings`)
  - Default maintenance cron: `*/5 * * * *` (every 5 minutes)

## Routes

Swagger:
/api/docs/
/admin/api/docs/

## Todos
- update readme
- add options for custom tables (like bricks)
- ~~have a tree view for entities~~
  - have easy-creation of children
- test admin routes
- connect dashboard data to backend
- check entities and respective columns - we need proper creation and not everything serialized.
- check env vars
- clippy
- custom field type (json with predefined content - like a preferences structure...)
- key-value-store
- relations 1:n + n:n
- admin swagger field definitions / constraints
- tests
- crons + refresh token deletion
- load/performance test binary
- typescript bindings
- anyhow in the entire repo
- run now file upload with different file types / formats
- DSL FE + be tests
- DSL readme
- admin easy default password warning (admin admin) -> hint in admin if default pw is being used
- add unique constraint to entity_definitions (FE + BE)
- uuid refactoring -> all in db
- toDef ->update action -> by key -> dropdown of existing one
- push all path / params / post body usages to dependency injection
- permission scheme testing and creation
  - entities: publish
  - workflows: activate
  - entity_definitions: publish
  - versions -> are included in read permissions
  - FE must react to those permissions
- notification / message system
  - messages
  - update on successfully run workflows
  - user requests permission(s)
  - default admin user not changed

Check DSL:

- map to entity
- map to field
- map to validation
- map to uri / source
- calculate (basic arithmetic)
- concatenation of string (conversion of int, float etc)

fixes:

- setting all fields for dynamic entities
- testing validations (+ tests)
- auth tests for all api routes
- further refactoring (FE / BE independently)
- tests (unit and integration) for dynamic entities (more)
- getting all entity types with fields and validations
- filter entities (by field and value) (validate against entity-definition)

## License Model

Draft of an idea

![licenses.png](docs/licenses.png)
