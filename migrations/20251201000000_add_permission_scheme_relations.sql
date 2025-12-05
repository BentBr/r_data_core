-- Add super_admin flag and many-to-many relationships for permission schemes
-- This migration adds:
-- 1. super_admin flag to admin_users table
-- 2. Junction table for admin_users <-> permission_schemes (many-to-many)
-- 3. Junction table for api_keys <-> permission_schemes (many-to-many)
-- 4. Removes old permission_scheme_uuid column from admin_users
-- 5. Removes old permissions JSONB column from api_keys

-- Add super_admin flag to admin_users table
ALTER TABLE admin_users
ADD COLUMN IF NOT EXISTS super_admin BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_admin_users_super_admin ON admin_users(super_admin);

-- Create junction table for admin_users and permission_schemes
CREATE TABLE IF NOT EXISTS admin_users_permission_schemes (
    user_uuid UUID NOT NULL REFERENCES admin_users(uuid) ON DELETE CASCADE,
    scheme_uuid UUID NOT NULL REFERENCES permission_schemes(uuid) ON DELETE CASCADE,
    PRIMARY KEY (user_uuid, scheme_uuid)
);

CREATE INDEX IF NOT EXISTS idx_admin_users_permission_schemes_user ON admin_users_permission_schemes(user_uuid);
CREATE INDEX IF NOT EXISTS idx_admin_users_permission_schemes_scheme ON admin_users_permission_schemes(scheme_uuid);

-- Create junction table for api_keys and permission_schemes
CREATE TABLE IF NOT EXISTS api_keys_permission_schemes (
    api_key_uuid UUID NOT NULL REFERENCES api_keys(uuid) ON DELETE CASCADE,
    scheme_uuid UUID NOT NULL REFERENCES permission_schemes(uuid) ON DELETE CASCADE,
    PRIMARY KEY (api_key_uuid, scheme_uuid)
);

CREATE INDEX IF NOT EXISTS idx_api_keys_permission_schemes_key ON api_keys_permission_schemes(api_key_uuid);
CREATE INDEX IF NOT EXISTS idx_api_keys_permission_schemes_scheme ON api_keys_permission_schemes(scheme_uuid);

-- Remove permissions JSONB column from api_keys if it exists
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'api_keys'
        AND column_name = 'permissions'
    ) THEN
        ALTER TABLE api_keys DROP COLUMN permissions;
    END IF;
END $$;

