-- Enable UUID extension and v7 function 
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create a function to automatically update the updated_at column
CREATE OR REPLACE FUNCTION update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
   NEW.updated_at = NOW();
   RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Entity Registry Table
CREATE TABLE IF NOT EXISTS entity_registry (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    entity_type VARCHAR(100) NOT NULL UNIQUE,
    display_name VARCHAR(255) NOT NULL,
    table_name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add auto-update trigger for entity_registry
DROP TRIGGER IF EXISTS set_timestamp_entity_registry ON entity_registry;
CREATE TRIGGER set_timestamp_entity_registry
BEFORE UPDATE ON entity_registry
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- Entity Versions Table
CREATE TABLE IF NOT EXISTS entity_versions (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    entity_uuid UUID NOT NULL,
    version INT NOT NULL,
    data JSONB NOT NULL,
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    comment TEXT,
    UNIQUE(entity_uuid, version)
);

CREATE INDEX IF NOT EXISTS idx_entity_versions_entity_uuid ON entity_versions(entity_uuid);

-- Class Definitions Table
CREATE TABLE IF NOT EXISTS class_definitions (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    entity_type VARCHAR(100) NOT NULL UNIQUE,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    group_name VARCHAR(100),
    allow_children BOOLEAN NOT NULL DEFAULT FALSE,
    icon VARCHAR(100),
    field_definitions JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_class_definitions_uuid ON class_definitions(uuid);

-- Entities Data Table
CREATE TABLE IF NOT EXISTS entities (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    path VARCHAR(255) NOT NULL,
    entity_type VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1,
    field_data JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entities_uuid ON entities(uuid);
CREATE INDEX IF NOT EXISTS idx_entities_path ON entities(path);
CREATE INDEX IF NOT EXISTS idx_entities_entity_type ON entities(entity_type);

-- Admin Users Table
CREATE TABLE IF NOT EXISTS admin_users (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    path TEXT NOT NULL DEFAULT '/users',
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    last_login TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_admin_users_username ON admin_users(username);
CREATE INDEX IF NOT EXISTS idx_admin_users_email ON admin_users(email);

-- Permission Schemes Table
CREATE TABLE IF NOT EXISTS permission_schemes (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    path TEXT NOT NULL DEFAULT '/permissions',
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    rules JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_permission_schemes_name ON permission_schemes(name);

-- API Keys Table
CREATE TABLE IF NOT EXISTS api_keys (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    path TEXT NOT NULL DEFAULT '/api-keys',
    user_uuid UUID NOT NULL REFERENCES admin_users(uuid) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    description TEXT,
    permissions JSONB,
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_uuid ON api_keys(user_uuid);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);

-- Notifications Table
CREATE TABLE IF NOT EXISTS notifications (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    path TEXT NOT NULL DEFAULT '/notifications',
    notification_type VARCHAR(50) NOT NULL,
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    recipient_uuid UUID,
    recipient_email TEXT,
    priority VARCHAR(20) NOT NULL DEFAULT 'normal',
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    scheduled_for TIMESTAMPTZ,
    sent_at TIMESTAMPTZ,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_notifications_recipient_uuid ON notifications(recipient_uuid);
CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);

-- Workflow Definitions Table
CREATE TABLE IF NOT EXISTS workflow_definitions (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    path TEXT NOT NULL DEFAULT '/workflows/definitions',
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    triggers JSONB NOT NULL,
    states JSONB NOT NULL,
    actions JSONB NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_workflow_definitions_name ON workflow_definitions(name);

-- Workflows Table
CREATE TABLE IF NOT EXISTS workflows (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    path TEXT NOT NULL DEFAULT '/workflows',
    definition_uuid UUID NOT NULL REFERENCES workflow_definitions(uuid) ON DELETE CASCADE,
    entity_uuid UUID,
    entity_type VARCHAR(100),
    current_state VARCHAR(100) NOT NULL,
    data JSONB NOT NULL DEFAULT '{}',
    history JSONB NOT NULL DEFAULT '[]',
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_workflows_definition_uuid ON workflows(definition_uuid);
CREATE INDEX IF NOT EXISTS idx_workflows_entity_uuid ON workflows(entity_uuid);
CREATE INDEX IF NOT EXISTS idx_workflows_status ON workflows(status);
