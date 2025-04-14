#!/bin/bash
set -e

# First, run the original entrypoint script with all arguments
/usr/local/bin/docker-entrypoint.sh "$@" &

# Wait more specifically for PostgreSQL to be ready
until pg_isready -h localhost -U postgres; do
  echo "Waiting for PostgreSQL to be ready..."
  sleep 1
done

# Wait for the database to start
sleep 5

# Create SQL function definition
cat > /tmp/uuid_v7.sql << 'EOF'
-- Enable the standard UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Function to generate a UUID v7
CREATE OR REPLACE FUNCTION uuid_generate_v7()
RETURNS uuid AS $$
DECLARE
    v_time bigint;
    v_timestamp varchar;
    v_hex varchar;
    v_result uuid;
BEGIN
    -- Get current time in milliseconds since epoch
    v_time := (extract(epoch from clock_timestamp()) * 1000)::bigint;
    
    -- Convert to hex - first 48 bits of UUID (12 hex chars)
    v_timestamp := lpad(to_hex(v_time), 12, '0');
    
    -- Create a UUID v4 and replace the first 12 chars with our timestamp
    v_hex := v_timestamp || substring(replace(uuid_generate_v4()::text, '-', ''), 13, 20);
    
    -- Set version to 7 (position 13 should be '7')
    v_hex := substring(v_hex, 1, 12) || '7' || substring(v_hex, 14, 19);
    
    -- Set variant bits to binary 10xx (position 17 should be '8', '9', 'a', or 'b')
    IF substring(v_hex, 17, 1) !~ '[89ab]' THEN
        v_hex := substring(v_hex, 1, 16) || '8' || substring(v_hex, 18, 15);
    END IF;
    
    -- Format with hyphens
    v_result := (substring(v_hex, 1, 8) || '-' || 
                 substring(v_hex, 9, 4) || '-' || 
                 substring(v_hex, 13, 4) || '-' || 
                 substring(v_hex, 17, 4) || '-' || 
                 substring(v_hex, 21, 12))::uuid;
    
    RETURN v_result;
END;
$$ LANGUAGE plpgsql;
EOF

# Install in postgres database
echo "Installing UUID v7 function in postgres database..."
psql -U postgres -f /tmp/uuid_v7.sql
echo "UUID v7 function installed in postgres database."

# Create rdata database if it doesn't exist
echo "Creating rdata database if it doesn't exist..."
psql -U postgres -c "CREATE DATABASE rdata WITH OWNER postgres;" || echo "Database rdata already exists."

# Install in rdata database
echo "Installing UUID v7 function in rdata database..."
psql -U postgres -d rdata -f /tmp/uuid_v7.sql
echo "UUID v7 function installed in rdata database."

# Keep the script running to avoid container exit
wait 