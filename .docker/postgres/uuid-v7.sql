-- First, enable the standard UUID extension in template1
CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;

-- Try to create the pg_uuidv7 extension if available
DO $$
BEGIN
  CREATE EXTENSION IF NOT EXISTS pg_uuidv7 WITH SCHEMA public;
  PERFORM uuid_generate_v7();
  RAISE NOTICE 'pg_uuidv7 extension successfully installed and working';
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'Could not create pg_uuidv7 extension: %', SQLERRM;
  RAISE NOTICE 'Will use SQL fallback implementation instead';
END $$;

-- Function to generate a UUID v7 (if extension not available)
CREATE OR REPLACE FUNCTION public.uuid_generate_v7()
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
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Install in template1 so all new databases get the function
\c template1

-- Add the uuid extension and function to template1
CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;

-- Create the function in template1
CREATE OR REPLACE FUNCTION public.uuid_generate_v7()
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
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Create a test to verify it's working
DO $$
BEGIN
    -- Test UUID v7 generation
    PERFORM uuid_generate_v7();
    RAISE NOTICE 'UUID v7 function is working correctly in template1';
EXCEPTION 
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Error testing UUID v7 function in template1: %', SQLERRM;
END $$; 