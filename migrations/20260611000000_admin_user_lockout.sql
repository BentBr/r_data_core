-- Add account-lockout tracking columns to admin_users.
-- status mirrors the UserStatus enum stored as VARCHAR ('Active' default).
ALTER TABLE admin_users
    ADD COLUMN IF NOT EXISTS status VARCHAR(32) NOT NULL DEFAULT 'Active',
    ADD COLUMN IF NOT EXISTS failed_login_attempts INTEGER NOT NULL DEFAULT 0;
