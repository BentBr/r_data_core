#!/bin/bash
set -e

# Wait for PostgreSQL to start
until pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB"; do
  echo "Waiting for PostgreSQL to start..."
  sleep 1
done

# Install UUID extensions
echo "Installing UUID v7 support..."
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
  -- Enable the standard UUID extension
  CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
  
  -- Try to create the pg_uuidv7 extension if available
  DO \$\$
  BEGIN
    CREATE EXTENSION IF NOT EXISTS pg_uuidv7;
    PERFORM uuid_generate_v7();
    RAISE NOTICE 'pg_uuidv7 extension successfully installed and working';
  EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'pg_uuidv7 extension not available, will install SQL implementation';
  END \$\$;
EOSQL

# Install our SQL implementation regardless
# If the extension works, this will be ignored due to "IF NOT EXISTS"
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" -f /docker-entrypoint-initdb.d/fallback-uuid-v7.sql

echo "UUID v7 support is now available" 