-- Add super_admin flag to roles table
-- This allows roles to grant super admin privileges to users
-- Note: This column is already included in the comprehensive schema, but kept for migration compatibility

ALTER TABLE roles
ADD COLUMN IF NOT EXISTS super_admin BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_roles_super_admin ON roles(super_admin);

