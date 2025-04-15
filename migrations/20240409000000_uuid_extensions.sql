-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create a function to generate UUID v7
CREATE OR REPLACE FUNCTION uuid_generate_v7()
RETURNS uuid
AS $$
DECLARE
    v_time timestamptz := clock_timestamp();
    v_secs bigint;
    v_msec bigint;
    v_usec bigint;
    v_nsec bigint;
    v_timestamp bigint;
    v_timestamp_hex varchar;
    v_random bytea;
    v_uuid uuid;
BEGIN
    -- Extract time components
    v_secs := EXTRACT(EPOCH FROM v_time);
    v_msec := EXTRACT(MILLISECONDS FROM v_time) % 1000;
    v_timestamp := (v_secs * 1000) + v_msec;
    
    -- Convert timestamp to hex and pad to 12 characters (48 bits)
    v_timestamp_hex := lpad(to_hex(v_timestamp), 12, '0');
    
    -- Generate 10 bytes (80 bits) of randomness
    v_random := gen_random_bytes(10);
    
    -- Combine the components to form a UUIDv7
    v_uuid := (
        v_timestamp_hex || 
        '7' || -- version 7
        substr(encode(v_random, 'hex'), 1, 3) || -- 12 bits of randomness, with highest bit set to 0 as per variant
        substr(encode(v_random, 'hex'), 4, 16) -- remaining randomness
    )::uuid;
    
    RETURN v_uuid;
END;
$$ LANGUAGE plpgsql VOLATILE;

COMMENT ON FUNCTION uuid_generate_v7() IS 'Generates a UUID version 7 (timestamp-based) following draft RFC'; 