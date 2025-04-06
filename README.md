# R Data Core

A robust backend for flexible data management with dynamic entities, workflows, and APIs.

## Features

- **Dynamic Entity System**: Create and manage entity classes and instances with customizable fields
- **Workflow Management**: Define and execute workflows for business processes
- **API Authentication**: Secure API access with JWT tokens and API keys
- **Redis Caching**: Fast caching for improved performance
- **PostgreSQL Database**: Reliable data storage
- **Migration System**: Structured database versioning

## Requirements

- Rust 1.75+
- PostgreSQL 14+
- Redis (optional, for caching)

## Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and update the values
3. Create a PostgreSQL database

```bash
createdb rdata
```

4. Run database migrations

```bash
cargo run --bin run_migrations
```

5. Start the server

```bash
cargo run
```

## Database Migrations

The application uses a structured migration system to manage database schema changes. Migrations are run automatically when the application starts, but can also be run manually:

```bash
cargo run --bin run_migrations
```

Each migration is tracked in the database, ensuring it is only applied once.

## Docker Setup

You can also run the application using Docker Compose:

```bash
docker-compose up -d
```

This will start PostgreSQL, Redis, and the R Data Core service.

## API Documentation

API documentation is available at `/api/docs` when the server is running.

## Cache Configuration

The application supports both in-memory caching and Redis caching. Redis is used if the `REDIS_URL` environment variable is set, otherwise, in-memory caching is used.

## License

MIT 