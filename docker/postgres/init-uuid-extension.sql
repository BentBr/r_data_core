-- First, enable the standard UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Then enable the UUIDv7 extension
CREATE EXTENSION IF NOT EXISTS pg_uuidv7;

-- Create a test to verify it's working
DO $$
BEGIN
    -- Test UUID v7 generation
    PERFORM uuid_generate_v7();
    RAISE NOTICE 'UUIDv7 extension is working correctly';
EXCEPTION 
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Error testing UUIDv7 extension: %', SQLERRM;
END $$; 