-- Add super_admin flag and many-to-many relationships for roles
-- This migration adds:
-- 1. super_admin flag to admin_users table
-- 2. Junction table for admin_users <-> roles (many-to-many)
-- 3. Junction table for api_keys <-> roles (many-to-many)

-- Add super_admin flag to admin_users table
ALTER TABLE admin_users
ADD COLUMN IF NOT EXISTS super_admin BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_admin_users_super_admin ON admin_users(super_admin);

-- Create junction table for admin_users and roles
CREATE TABLE IF NOT EXISTS user_roles (
    user_uuid UUID NOT NULL REFERENCES admin_users(uuid) ON DELETE CASCADE,
    role_uuid UUID NOT NULL REFERENCES roles(uuid) ON DELETE CASCADE,
    PRIMARY KEY (user_uuid, role_uuid)
);

CREATE INDEX IF NOT EXISTS idx_user_roles_user ON user_roles(user_uuid);
CREATE INDEX IF NOT EXISTS idx_user_roles_role ON user_roles(role_uuid);

-- Create junction table for api_keys and roles
CREATE TABLE IF NOT EXISTS api_key_roles (
    api_key_uuid UUID NOT NULL REFERENCES api_keys(uuid) ON DELETE CASCADE,
    role_uuid UUID NOT NULL REFERENCES roles(uuid) ON DELETE CASCADE,
    PRIMARY KEY (api_key_uuid, role_uuid)
);

CREATE INDEX IF NOT EXISTS idx_api_key_roles_key ON api_key_roles(api_key_uuid);
CREATE INDEX IF NOT EXISTS idx_api_key_roles_role ON api_key_roles(role_uuid);

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

