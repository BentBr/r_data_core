#!/bin/bash
set -e

# Define the SQL for the UUID v7 function
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

# Create the function in the postgres database
psql -U postgres -f /tmp/uuid_v7.sql

# Test the function
psql -U postgres -c "SELECT uuid_generate_v7() AS test_uuid;"

echo "UUID v7 function installed successfully!" 