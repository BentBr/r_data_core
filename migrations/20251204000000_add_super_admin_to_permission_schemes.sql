-- Add super_admin flag to permission_schemes table
-- This allows permission schemes to grant super admin privileges to users

ALTER TABLE permission_schemes
ADD COLUMN IF NOT EXISTS super_admin BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_permission_schemes_super_admin ON permission_schemes(super_admin);

