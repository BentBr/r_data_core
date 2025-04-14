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

#### Authentication
- `POST /api/auth/login` - Login with username and password
- `POST /api/auth/refresh` - Refresh JWT token
- `POST /api/auth/logout` - Logout and invalidate token

#### Admin API
- `GET /api/admin/users` - List admin users
- `POST /api/admin/users` - Create admin user
- `GET /api/admin/users/{id}` - Get admin user details
- `PUT /api/admin/users/{id}` - Update admin user
- `DELETE /api/admin/users/{id}` - Delete admin user

#### API Keys
- `GET /api/admin/api-keys` - List API keys
- `POST /api/admin/api-keys` - Create API key
- `GET /api/admin/api-keys/{id}` - Get API key details
- `PUT /api/admin/api-keys/{id}` - Update API key
- `DELETE /api/admin/api-keys/{id}` - Delete API key

#### Class Definitions
- `GET /api/admin/class-definitions` - List class definitions
- `POST /api/admin/class-definitions` - Create class definition
- `GET /api/admin/class-definitions/{id}` - Get class definition
- `PUT /api/admin/class-definitions/{id}` - Update class definition
- `DELETE /api/admin/class-definitions/{id}` - Delete class definition

#### Dynamic Entities
- `GET /api/entities/{type}` - List entities of a type
- `POST /api/entities/{type}` - Create entity
- `GET /api/entities/{type}/{id}` - Get entity details
- `PUT /api/entities/{type}/{id}` - Update entity
- `DELETE /api/entities/{type}/{id}` - Delete entity

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

For more information about our PostgreSQL setup with UUID v7, see the [documentation](./docker/postgres/README.md).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 