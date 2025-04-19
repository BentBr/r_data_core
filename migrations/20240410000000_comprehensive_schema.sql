-- Comprehensive schema for RData Core
-- All TIMESTAMPTZ fields are interacted with using time::OffsetDateTime in application code
-- Note: This migration assumes PostgreSQL 13+ with uuid-ossp extension

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

-- Enhanced Entity Registry Table (central metadata store for all entities)
CREATE TABLE IF NOT EXISTS entities_registry (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    entity_type VARCHAR(100) NOT NULL,
    path VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

-- Create index on entity_type for faster lookups
CREATE INDEX IF NOT EXISTS idx_entities_registry_entity_type ON entities_registry(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_registry_path ON entities_registry(path);

-- Add auto-update trigger for entity_registry
DROP TRIGGER IF EXISTS set_timestamp_entities_registry ON entities_registry;
CREATE TRIGGER set_timestamp_entities_registry
BEFORE UPDATE ON entities_registry
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- Entity Versions Table
CREATE TABLE IF NOT EXISTS entities_versions (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    entity_uuid UUID NOT NULL,
    version INT NOT NULL,
    data JSONB NOT NULL,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    comment TEXT,
    UNIQUE(entity_uuid, version)
);

CREATE INDEX IF NOT EXISTS idx_entities_versions_entity_uuid ON entities_versions(entity_uuid);

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
    created_by UUID NOT NULL,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_class_definitions_uuid ON class_definitions(uuid);

-- Helper function to create a view for an entity type
CREATE OR REPLACE FUNCTION create_entity_view(entity_type TEXT) RETURNS void AS $$
DECLARE
    view_name TEXT;
    entity_table TEXT;
    registry_join TEXT;
    view_query TEXT;
    col_record RECORD;
    column_list TEXT := '';
    separator TEXT := '';
    existing_view BOOLEAN;
BEGIN
    -- Set the view name based on the entity type
    view_name := 'entity_' || entity_type || '_view';
    entity_table := 'entity_' || entity_type;
    
    -- Check if view already exists
    SELECT EXISTS(
        SELECT FROM information_schema.views 
        WHERE table_schema = 'public' AND table_name = view_name
    ) INTO existing_view;
    
    -- Log start of view creation/update
    RAISE NOTICE 'Creating or replacing view % for entity table %', view_name, entity_table;
    
    -- Only try to drop the view if it exists
    IF existing_view THEN
        -- Drop the view if it exists
        EXECUTE 'DROP VIEW IF EXISTS ' || view_name || ' CASCADE';
        RAISE NOTICE 'Dropped existing view %', view_name;
    END IF;
    
    -- Verify that entity table exists
    PERFORM 1 FROM information_schema.tables 
    WHERE table_schema = 'public' AND table_name = entity_table;
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Entity table % does not exist', entity_table;
    END IF;

    -- Get columns from entity table, excluding uuid
    FOR col_record IN
        EXECUTE format('
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = ''public'' AND table_name = %L
            AND column_name <> ''uuid''
            ORDER BY ordinal_position
        ', entity_table)
        LOOP
            RAISE NOTICE 'Including column % from entity table', col_record.column_name;
            column_list := column_list || separator || 'e.' || col_record.column_name;
            separator := ', ';
        END LOOP;

    -- Create the SQL for joining with the registry, explicitly listing all columns
    -- The key is to only include uuid from registry (r.uuid) and exclude it from entity table
    registry_join := 'SELECT r.uuid, r.entity_type, r.path, r.created_at, r.updated_at, ' ||
                     'r.created_by, r.updated_by, r.published, r.version';

    -- Only add entity columns if there are any (other than uuid)
    IF column_list <> '' THEN
        registry_join := registry_join || ', ' || column_list;
    END IF;

    registry_join := registry_join ||
                     ' FROM entities_registry r ' ||
                     'LEFT JOIN ' || entity_table || ' e ON r.uuid = e.uuid ' ||
                     'WHERE r.entity_type = ''' || entity_type || '''';

    -- Create the view
    view_query := 'CREATE VIEW ' || view_name || ' AS ' || registry_join;

    -- Log the executed query for debugging
    RAISE NOTICE 'Creating view with query: %', view_query;

    EXECUTE view_query;
    
    -- Verify view was created properly
    PERFORM column_name FROM information_schema.columns
    WHERE table_schema = 'public' AND table_name = view_name;
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Failed to create view % properly', view_name;
    END IF;
    
    -- Log column list for debugging
    FOR col_record IN
        EXECUTE format('
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = ''public'' AND table_name = %L
            ORDER BY ordinal_position
        ', view_name)
        LOOP
            RAISE NOTICE 'View % contains column %', view_name, col_record.column_name;
        END LOOP;

    -- Grant permissions on the view
    EXECUTE 'GRANT SELECT ON ' || view_name || ' TO PUBLIC';
    
    RAISE NOTICE 'Successfully created/updated view %', view_name;
END;
$$ LANGUAGE plpgsql;

-- Helper function to create or update an entity-specific table
CREATE OR REPLACE FUNCTION create_entity_table_and_view(entity_type TEXT)
    RETURNS VOID AS $$
DECLARE
    table_name TEXT;
BEGIN
    table_name := 'entity_' || entity_type;

    -- Create the basic entity table if it doesn't exist
    EXECUTE format('
        CREATE TABLE IF NOT EXISTS %I (
            uuid UUID PRIMARY KEY REFERENCES entities_registry(uuid) ON DELETE CASCADE
        )',
                   table_name);

    -- Create or update the entity view
    PERFORM create_entity_view(entity_type);
END;
$$ LANGUAGE plpgsql;

-- Refresh any existing entity views
DO $$
    DECLARE
        r RECORD;
    BEGIN
        FOR r IN SELECT entity_type FROM class_definitions
            LOOP
                PERFORM create_entity_view(r.entity_type);
            END LOOP;
    END $$;

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
    created_by UUID NOT NULL,
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
    created_by UUID NOT NULL,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE (user_uuid, name)
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
    recipient_uuid UUID NOT NULL,
    recipient_email TEXT,
    priority VARCHAR(20) NOT NULL DEFAULT 'normal',
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    scheduled_for TIMESTAMPTZ,
    sent_at TIMESTAMPTZ,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL,
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
    created_by UUID NOT NULL,
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_workflows_definition_uuid ON workflows(definition_uuid);
CREATE INDEX IF NOT EXISTS idx_workflows_entity_uuid ON workflows(entity_uuid);
CREATE INDEX IF NOT EXISTS idx_workflows_status ON workflows(status);

-- Trigger to create entity views when a new class definition is inserted
CREATE OR REPLACE FUNCTION create_entity_view_on_class_insert()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM create_entity_table_and_view(NEW.entity_type);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_create_entity_view ON class_definitions;
CREATE TRIGGER trigger_create_entity_view
AFTER INSERT ON class_definitions
FOR EACH ROW
EXECUTE FUNCTION create_entity_view_on_class_insert();

-- Create entity tables and views for existing class definitions
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT entity_type FROM class_definitions
    LOOP
        PERFORM create_entity_table_and_view(r.entity_type);
    END LOOP;
END $$;
