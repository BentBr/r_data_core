#!/bin/bash
set -e

# Wait for PostgreSQL to start
until pg_isready; do
  echo "Waiting for PostgreSQL to start..."
  sleep 1
done

echo "Installing UUID v7 function in postgres database..."
psql -U postgres -d postgres -f /docker-entrypoint-initdb.d/01-uuid-v7.sql

# Get list of all databases excluding templates
DATABASES=$(psql -U postgres -t -c "SELECT datname FROM pg_database WHERE datname NOT IN ('template0', 'template1', 'postgres');")

echo "Installing UUID v7 function in all other databases..."

# Install in each database
for DB in $DATABASES; do
  echo "Installing in database: $DB"
  psql -U postgres -d "$DB" -f /docker-entrypoint-initdb.d/01-uuid-v7.sql
done

echo "UUID v7 function installation completed!" 