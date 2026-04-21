-- System log enums
CREATE TYPE system_log_status AS ENUM ('success', 'failed', 'pending');
CREATE TYPE system_log_type AS ENUM ('email_sent', 'entity_created', 'entity_updated', 'entity_deleted', 'auth_event');
CREATE TYPE system_log_resource_type AS ENUM ('email', 'admin_user', 'role', 'workflow', 'entity_definition', 'email_template');

-- System logs table
CREATE TABLE system_logs (
    uuid          UUID PRIMARY KEY DEFAULT uuidv7(),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by    UUID,
    status        system_log_status NOT NULL,
    log_type      system_log_type NOT NULL,
    resource_type system_log_resource_type NOT NULL,
    resource_uuid UUID,
    summary       TEXT NOT NULL,
    details       JSONB
);

CREATE INDEX idx_system_logs_created_at ON system_logs(created_at DESC);
CREATE INDEX idx_system_logs_log_type ON system_logs(log_type);
CREATE INDEX idx_system_logs_resource_type ON system_logs(resource_type);
