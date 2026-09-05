-- Add account-lockout tracking columns to admin_users.
-- `status` is a dedicated Postgres enum (mirrors the Rust `UserStatus`),
-- snake_case to match the other enums in the schema.
CREATE TYPE admin_user_status AS ENUM ('active', 'inactive', 'locked', 'pending_activation');

ALTER TABLE admin_users
    ADD COLUMN IF NOT EXISTS status admin_user_status NOT NULL DEFAULT 'active',
    ADD COLUMN IF NOT EXISTS failed_login_attempts INTEGER NOT NULL DEFAULT 0;
