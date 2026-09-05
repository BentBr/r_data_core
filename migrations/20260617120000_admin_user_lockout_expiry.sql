-- Expiring account lockouts: a lock set by failed logins lifts itself once
-- `locked_until` passes, so an attacker cannot park an admin account in the
-- Locked state indefinitely. NULL means the lock was set by an operator (or
-- there is no lock) and only an operator can clear it.
ALTER TABLE admin_users
    ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ;
