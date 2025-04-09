#!/bin/bash
set -e

# Function to check if we can create the pg_uuidv7 extension
function check_extension() {
  psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    DO \$\$
    BEGIN
      CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
      CREATE EXTENSION IF NOT EXISTS pg_uuidv7;
      PERFORM uuid_generate_v7();
      RAISE NOTICE 'pg_uuidv7 extension successfully installed and working';
    EXCEPTION WHEN OTHERS THEN
      RAISE NOTICE 'Could not create pg_uuidv7 extension: %', SQLERRM;
      RAISE NOTICE 'Will use SQL fallback implementation instead';
      RETURN;
    END \$\$;
EOSQL
}

# Function to install the SQL fallback
function install_fallback() {
  echo "Installing SQL fallback implementation for uuid_generate_v7"
  psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" -f /docker-entrypoint-initdb.d/fallback-uuid-v7.sql
}

# Wait for PostgreSQL to start
until pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB"; do
  echo "Waiting for PostgreSQL to start..."
  sleep 1
done

# Try to create the extension
echo "Attempting to create pg_uuidv7 extension..."
if ! check_extension; then
  echo "Failed to create pg_uuidv7 extension, using SQL fallback"
  install_fallback
else
  echo "Successfully installed pg_uuidv7 extension"
fi

echo "UUID v7 support is now available" 