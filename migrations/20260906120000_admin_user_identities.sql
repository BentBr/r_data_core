-- External identities (OIDC) linked to admin users.
--
-- Identity is keyed on (provider, subject) — the issuer URL and the provider's
-- `sub` claim — and never on email. Email is mutable and re-assignable at most
-- providers, so keying on it would let anyone able to register an address at a
-- trusted issuer inherit the matching RDataCore account.
--
-- `provider` is stored per row rather than assumed global, so supporting more
-- than one issuer later needs no migration.

CREATE TABLE IF NOT EXISTS admin_user_identities (
    uuid UUID PRIMARY KEY DEFAULT uuidv7(),
    provider TEXT NOT NULL,
    subject TEXT NOT NULL,
    admin_user_uuid UUID NOT NULL REFERENCES admin_users(uuid) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    CONSTRAINT admin_user_identities_provider_subject_key UNIQUE (provider, subject)
);

CREATE INDEX IF NOT EXISTS idx_admin_user_identities_user
    ON admin_user_identities(admin_user_uuid);

-- Marks accounts created through SSO.
--
-- These carry an unusable password hash, and must be barred from every
-- password-based path: login, forgot-password, reset-password, and the
-- password fields on admin user CRUD. Without the flag, SSO becomes a way to
-- create password accounts and forgot-password becomes a way to set one.
ALTER TABLE admin_users
    ADD COLUMN IF NOT EXISTS is_sso_provisioned BOOLEAN NOT NULL DEFAULT FALSE;
