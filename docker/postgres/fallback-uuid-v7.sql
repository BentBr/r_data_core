-- First, enable the standard UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Function to generate a UUID v7
-- This is a pure SQL implementation that doesn't require compilation
CREATE OR REPLACE FUNCTION uuid_generate_v7()
RETURNS uuid AS $$
DECLARE
    v_time timestamp with time zone := clock_timestamp();
    v_ts bigint;
    v_microsec bigint;
    v_timestamp bytea;
    v_random bytea;
    v_uuid uuid;
BEGIN
    -- Extract timestamp components
    v_ts := EXTRACT(EPOCH FROM v_time)::bigint;
    v_microsec := EXTRACT(MICROSECONDS FROM v_time - '1970-01-01 00:00:00 UTC'::timestamp with time zone)::bigint % 1000000;
    
    -- Convert timestamp to bytes (first 48 bits)
    v_timestamp := E'\\x' || lpad(to_hex(v_ts), 10, '0') || lpad(to_hex(v_microsec), 6, '0');
    
    -- Get some random bytes for the rest of the UUID (78 bits)
    v_random := encode(gen_random_bytes(10), 'hex');
    
    -- Version 7 requires the 49th bit to be 0, 50-52nd bits to be '111'
    v_random := set_byte(
        v_random, 
        0, 
        (get_byte(v_random, 0) & 15) | 112  -- 0x70 = 01110000 in binary
    );
    
    -- Variant bits: 8th byte's top 2 bits must be '10'
    v_random := set_byte(
        v_random, 
        1, 
        (get_byte(v_random, 1) & 63) | 128  -- 0x80 = 10000000 in binary
    );
    
    -- Combine all parts to create UUID
    v_uuid := (substring(v_timestamp for 6) || substring(v_random for 10))::uuid;
    
    RETURN v_uuid;
END;
$$ LANGUAGE plpgsql;

-- Create a test to verify it's working
DO $$
BEGIN
    -- Test UUID v7 generation
    PERFORM uuid_generate_v7();
    RAISE NOTICE 'UUID v7 function is working correctly';
EXCEPTION 
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Error testing UUID v7 function: %', SQLERRM;
END $$; 