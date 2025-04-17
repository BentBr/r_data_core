# R Data Core

A robust backend for flexible data management with dynamic entity system, workflow management, API authentication, Redis caching, PostgreSQL database support, and migration system.

## Features

- Dynamic entity system for flexible data modeling
- Class definitions with customizable fields
- Entity registry with field validation
- API authentication (JWT and API key)
- Redis caching
- PostgreSQL database support
- Migration system
- API documentation at `/api/docs`

## Database Schema

The project uses a dynamic entity model with the following key tables:

- `class_definitions`: Defines entity types with their field definitions
- `entities_registry`: Stores all entities with their field data in a JSONB column
- `entity_versions`: Tracks changes to entities for versioning
- `entity_registry`: Registry of entity types (metadata)
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


renew database:
```bash
docker compose down -v && docker compose up -d redis postgres && sleep 7 && cargo sqlx migrate run
```

update sqlx:
```bash
cargo sqlx prepare
```


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

- `DATABASE_URL` - PostgreSQL connection URL
- `DATABASE_MAX_CONNECTIONS` - Maximum database connections
- `SERVER_HOST` - Server host address
- `SERVER_PORT` - Server port
- `JWT_SECRET` - Secret key for JWT tokens
- `JWT_EXPIRATION` - JWT token expiration in seconds
- `REDIS_URL` - Redis connection URL (optional)
- `CACHE_ENABLED` - Enable caching (true/false)
- `CACHE_TTL` - Cache TTL in seconds
- `RUST_LOG` - Logging level (info/debug/error)

## Cache Configuration

The application supports both in-memory caching and Redis caching. By default, Redis caching is enabled when the `REDIS_URL` environment variable is set.

## UUID v7 Support

This project uses UUID v7 for generating primary keys. Our Docker setup includes a custom PostgreSQL image that automatically installs the necessary extension:

- When using Docker: UUID v7 support is automatically provided by our custom PostgreSQL image
- For local development without Docker: You'll need to install the `pg_uuidv7` extension

### Installing UUID v7 Extension for Local Development

If you're developing locally without Docker, follow these steps to install the UUID v7 extension:

```bash
# Clone the pg_uuidv7 repository
git clone https://github.com/fboulnois/pg_uuidv7.git
cd pg_uuidv7

# Build and install the extension
make
sudo make install

# Enable the extension in your database
psql -U postgres -d your_database_name -c "CREATE EXTENSION pg_uuidv7;"
```

For more information about our PostgresSQL setup with UUID v7, see the [documentation](./docker/postgres/README.md).

## Entity System

The R Data Core provides a flexible system for defining and working with dynamic entities. This allows you to create custom data structures at runtime through the API rather than needing to modify the application code.

### Key Concepts

#### Class Definitions

A **Class Definition** is a schema that defines the structure of an entity type. It includes:
- A unique identifier
- An entity type name (which becomes the table name)
- A set of field definitions
- Metadata about the entity type (description, display name, etc.)
- Schema information for database representation

When a class definition is created or updated, the system automatically generates the necessary database tables and columns to store entities of that type.

You can find example json files in [example files](./.json_examples)

#### Fields

Each **Field Definition** within a class specifies:
- Field name and display name
- Data type (String, Integer, Boolean, DateTime, etc.)
- Validation rules (required, min/max length, patterns, etc.)
- UI settings for rendering the field in client applications
- Database storage settings (indexed, filterable, etc.)

#### Dynamic Entities

A **Dynamic Entity** is an actual data object that follows the structure defined by a class definition. Entities can be created, updated, queried, and deleted through the API. Each entity instance includes:
- Standard system fields (UUID, creation timestamp, etc.)
- The custom fields defined in its class definition
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

All class definition endpoints are secured with JWT authentication and require admin privileges:

- `GET /admin/api/v1/class-definitions` - List all class definitions
- `GET /admin/api/v1/class-definitions/{uuid}` - Get a specific class definition
- `POST /admin/api/v1/class-definitions` - Create a new class definition 
- `PUT /admin/api/v1/class-definitions/{uuid}` - Update an existing class definition
- `DELETE /admin/api/v1/class-definitions/{uuid}` - Delete a class definition

Entity data can be manipulated through the public API endpoints (documentation pending).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 


Routes:
Swagger:
/api/docs/
/admin/api/docs/


Todos:
- ~~fix sqlx topic~~
- ~~fix swagger~~
- add routes
  - ~~auth~~
  - ~~class-definitions~~
  - workflows
    - export
      - json, xml, csv
      - graphql
    - import
      - json, xml, csv
      - web
      - webhooks
    - manipulate data
  - versions
  - entities
    - crud
  - permissions
- update readme
- add options for custom tables (like bricks)
- test admin routes
- check entities and respective columns - we need proper creation and not everything serialized.
- check env vars
- clippy
- custom field type (json with predefined content - like a preferences structure...)
- key-value-store
- relations 1:n + n:n

delete tables when class-definition is deleted
cleanup old tables command

