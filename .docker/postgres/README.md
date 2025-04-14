# Custom PostgreSQL with UUID v7 Support

This directory contains the necessary files to build a custom PostgreSQL image with UUID v7 support, which is needed by the R Data Core application.

## Features

- Based on official PostgreSQL 16 Alpine image
- Provides UUID v7 support through one of two methods:
  1. Primary: The [pg_uuidv7](https://github.com/fboulnois/pg_uuidv7) extension (if compilation succeeds)
  2. Fallback: A pure SQL implementation (if extension compilation fails)
- Automatically enables the appropriate UUID v7 implementation during initialization

## How It Works

Our setup includes a robust mechanism to ensure UUID v7 functionality is available:

1. During the image build, we attempt to compile and install the pg_uuidv7 extension
2. At container startup, we verify if the extension is working correctly
3. If the extension fails for any reason, we automatically fall back to our pure SQL implementation
4. The `uuid_generate_v7()` function will be available in either case - your application code doesn't need to change

## How to Use

When you run the application with Docker Compose, this custom image will be built and used automatically:

```bash
docker compose up -d
```

## Using UUID v7 in SQL

Once the database is running, you can use the `uuid_generate_v7()` function in your SQL queries:

```sql
-- Generate a new UUID v7
SELECT uuid_generate_v7();

-- Create a table with UUID v7 as the primary key
CREATE TABLE example (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
  name TEXT
);

-- Insert a row with an automatically generated UUID v7
INSERT INTO example (name) VALUES ('test');
```

## Benefits of UUID v7

UUID v7 provides several advantages over traditional UUIDs:

1. **Time-ordered**: UUID v7 values are sortable by time, unlike random UUIDs
2. **High performance**: Eliminates the "hot spots" in indexes that can occur with sequential IDs
3. **Security**: Makes it difficult to guess IDs while maintaining time ordering
4. **Uniqueness**: Guarantees uniqueness across distributed systems

## Troubleshooting

If you encounter any issues with the UUID v7 functionality, you can:

1. Check which implementation is being used:
   ```sql
   SELECT pg_extension.extname 
   FROM pg_extension 
   WHERE extname = 'pg_uuidv7';
   ```
   If this returns a row, you're using the native extension. If not, you're using the SQL fallback.

2. Verify the function is working correctly:
   ```sql
   SELECT uuid_generate_v7();
   ```
   This should return a valid UUID regardless of which implementation is active.

3. Check the container logs:
   ```bash
   docker logs rdata_postgres
   ```
   Look for messages about the pg_uuidv7 extension or the SQL fallback implementation. 