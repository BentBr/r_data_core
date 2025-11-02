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
    entity_key VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL,
    updated_by UUID,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE (path, entity_key)
);

-- Create index on entity_type for faster lookups
CREATE INDEX IF NOT EXISTS idx_entities_registry_entity_type ON entities_registry(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_registry_path ON entities_registry(path);
CREATE INDEX IF NOT EXISTS idx_entities_registry_key ON entities_registry(entity_key);

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

-- Entity Definitions Table
CREATE TABLE IF NOT EXISTS entity_definitions (
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

CREATE INDEX IF NOT EXISTS idx_entity_definitions_uuid ON entity_definitions(uuid);

-- Create a logging function that can be used to debug UUID issues
CREATE OR REPLACE FUNCTION log_uuid_debug(uuid_val UUID)
RETURNS UUID AS $$
BEGIN
    RAISE NOTICE 'UUID debug: %', uuid_val;
    RETURN uuid_val;
END;
$$ LANGUAGE plpgsql;

-- Helper function to create or update an entity-specific table
CREATE OR REPLACE FUNCTION create_entity_table_and_view(entity_type_param TEXT)
RETURNS VOID AS $$
DECLARE
    table_name TEXT;
    view_name TEXT;
    entity_def RECORD;
    field_record RECORD;
    column_record RECORD;
    field_names TEXT[] := ARRAY[]::TEXT[];
    column_name TEXT;
    field_name TEXT;
    field_type TEXT;
    sql_type TEXT;
    drop_sql TEXT;
    view_exists BOOLEAN;
    col_exists BOOLEAN;
    trigger_name TEXT;
    entity_field_list TEXT := '';
    entity_field_values TEXT := '';
    entity_update_list TEXT := '';
    entity_field_separator TEXT := '';
    trigger_sql TEXT;
BEGIN
    -- Set the table and view names
    table_name := 'entity_' || lower(entity_type_param);
    view_name := table_name || '_view';

    -- Get the entity definition for this entity type
    SELECT * INTO entity_def FROM entity_definitions WHERE entity_type = entity_type_param;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'No entity definition found for entity type %', entity_type_param;
    END IF;

    -- Check if view exists before attempting to drop it
    EXECUTE format('
        SELECT EXISTS (
            SELECT FROM information_schema.views
            WHERE table_schema = ''public''
            AND table_name = %L
        )', view_name) INTO view_exists;

    -- Drop the view if it exists - do this first to avoid dependency issues
    IF view_exists THEN
        EXECUTE format('DROP VIEW IF EXISTS %I CASCADE', view_name);
        RAISE NOTICE 'Dropped existing view %', view_name;
    END IF;

    -- Extract field names now to avoid issues later
    FOR field_record IN
        SELECT jsonb_array_elements(entity_def.field_definitions) AS field
    LOOP
        field_name := lower(field_record.field->>'name');
        field_names := array_append(field_names, field_name);
    END LOOP;

    RAISE NOTICE 'Field names from entity definition: %', field_names;

    -- Create the table if it doesn't exist
    EXECUTE format('
        CREATE TABLE IF NOT EXISTS %I (
            uuid UUID PRIMARY KEY REFERENCES entities_registry(uuid) ON DELETE CASCADE
        )',
        table_name);

    -- Get existing columns
    FOR column_record IN
        EXECUTE format('
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = ''public'' AND table_name = %L
            AND column_name <> ''uuid''
        ', table_name)
    LOOP
        -- Check if this column exists in the field definitions
        column_name := lower(column_record.column_name);
        IF column_name <> ALL(field_names) AND column_name NOT IN ('created_at', 'updated_at', 'created_by', 'updated_by', 'published', 'version', 'path') THEN
            drop_sql := format('ALTER TABLE %I DROP COLUMN IF EXISTS %I',
                              table_name, column_name);
            RAISE NOTICE 'Dropping column: %', drop_sql;
            EXECUTE drop_sql;
        END IF;
    END LOOP;

    -- Add columns from field definitions
    FOREACH field_name IN ARRAY field_names
    LOOP
        -- Find matching field record
        SELECT field FROM (
            SELECT jsonb_array_elements(entity_def.field_definitions) AS field
        ) AS fields
        WHERE lower(field->>'name') = field_name
        INTO field_record;

        IF field_record IS NULL THEN
            CONTINUE;  -- Skip if not found
        END IF;

        field_type := field_record.field->>'field_type';

        -- Map field types to SQL types
        CASE field_type
            WHEN 'String' THEN sql_type := 'VARCHAR(255)';
            WHEN 'Text' THEN sql_type := 'TEXT';
            WHEN 'Wysiwyg' THEN sql_type := 'TEXT';
            WHEN 'Integer' THEN sql_type := 'INTEGER';
            WHEN 'Float' THEN sql_type := 'DOUBLE PRECISION';
            WHEN 'Boolean' THEN sql_type := 'BOOLEAN';
            WHEN 'DateTime' THEN sql_type := 'TIMESTAMPTZ';
            WHEN 'Date' THEN sql_type := 'DATE';
            WHEN 'Object' THEN sql_type := 'JSONB';
            WHEN 'Array' THEN sql_type := 'JSONB';
            WHEN 'Json' THEN sql_type := 'JSONB';
            WHEN 'Uuid' THEN sql_type := 'UUID';
            WHEN 'ManyToOne' THEN sql_type := 'UUID';
            WHEN 'ManyToMany' THEN sql_type := 'JSONB';
            WHEN 'Select' THEN sql_type := 'VARCHAR(100)';
            WHEN 'MultiSelect' THEN sql_type := 'JSONB';
            WHEN 'Image' THEN sql_type := 'VARCHAR(255)';
            WHEN 'File' THEN sql_type := 'VARCHAR(255)';
            ELSE sql_type := 'TEXT';
        END CASE;

        -- Check if column exists first to handle type changes appropriately
        EXECUTE format('
            SELECT EXISTS (
                SELECT FROM information_schema.columns
                WHERE table_schema = ''public''
                AND table_name = %L
                AND column_name = %L
            )
        ', table_name, field_name) INTO col_exists;

        IF col_exists THEN
            -- For existing columns that need type changes, handle with data preservation
            BEGIN
                -- Check the current type
                DECLARE
                    current_type TEXT;
                    alter_sql TEXT;
                    temp_col_name TEXT;
                BEGIN
                    EXECUTE format('
                        SELECT data_type FROM information_schema.columns
                        WHERE table_schema = ''public''
                        AND table_name = %L
                        AND column_name = %L
                    ', table_name, field_name) INTO current_type;

                    -- If type needs to change, try to do it safely
                    IF current_type IS DISTINCT FROM sql_type THEN
                        -- Try direct type cast first
                        BEGIN
                            alter_sql := format('ALTER TABLE %I ALTER COLUMN %I TYPE %s',
                                              table_name, field_name, sql_type);
                            EXECUTE alter_sql;
                            RAISE NOTICE 'Safely changed column % type from % to % with ALTER COLUMN',
                                      field_name, current_type, sql_type;
                        EXCEPTION WHEN OTHERS THEN
                            -- If direct cast fails, use temporary column approach
                            RAISE NOTICE 'Direct type conversion failed: %', SQLERRM;

                            -- Create a temporary column with new type
                            temp_col_name := field_name || '_new';
                            EXECUTE format('ALTER TABLE %I ADD COLUMN %I %s',
                                          table_name, temp_col_name, sql_type);

                            -- Try to copy data with explicit cast
                            BEGIN
                                EXECUTE format('UPDATE %I SET %I = %I::%s',
                                              table_name, temp_col_name, field_name, sql_type);

                                -- Drop old column
                                EXECUTE format('ALTER TABLE %I DROP COLUMN %I',
                                              table_name, field_name);

                                -- Rename temp column to original name
                                EXECUTE format('ALTER TABLE %I RENAME COLUMN %I TO %I',
                                              table_name, temp_col_name, field_name);

                                RAISE NOTICE 'Changed column % type from % to % using temporary column with data preserved',
                                          field_name, current_type, sql_type;
                            EXCEPTION WHEN OTHERS THEN
                                -- If casting fails, try without casting
                                RAISE NOTICE 'Cast conversion failed: %', SQLERRM;
                                BEGIN
                                    -- For some compatible types, we can try without explicit cast
                                    EXECUTE format('UPDATE %I SET %I = %I',
                                                  table_name, temp_col_name, field_name);

                                    -- Drop old column
                                    EXECUTE format('ALTER TABLE %I DROP COLUMN %I',
                                                  table_name, field_name);

                                    -- Rename temp column to original name
                                    EXECUTE format('ALTER TABLE %I RENAME COLUMN %I TO %I',
                                                  table_name, temp_col_name, field_name);

                                    RAISE NOTICE 'Changed column % type from % to % using temporary column with basic conversion',
                                              field_name, current_type, sql_type;
                                EXCEPTION WHEN OTHERS THEN
                                    -- If all attempts fail, drop the temporary column and use traditional approach
                                    RAISE NOTICE 'All conversion attempts failed: %', SQLERRM;
                                    EXECUTE format('ALTER TABLE %I DROP COLUMN IF EXISTS %I',
                                                  table_name, temp_col_name);

                                    -- Last resort: replace column (data will be lost)
                                    EXECUTE format('ALTER TABLE %I DROP COLUMN %I',
                                                  table_name, field_name);
                                    EXECUTE format('ALTER TABLE %I ADD COLUMN %I %s',
                                                  table_name, field_name, sql_type);

                                    RAISE NOTICE 'Unable to preserve data. Changed column % type from % to % with data loss',
                                              field_name, current_type, sql_type;
                                END;
                            END;
                        END;
                    END IF;
                END;
            EXCEPTION WHEN OTHERS THEN
                RAISE NOTICE 'Error handling column type change: %', SQLERRM;
            END;
        ELSE
            -- Add column if it doesn't exist
            EXECUTE format('ALTER TABLE %I ADD COLUMN IF NOT EXISTS %I %s', table_name, field_name, sql_type);
            RAISE NOTICE 'Added new column % with type %', field_name, sql_type;
        END IF;
    END LOOP;

    -- Now build field lists for views and triggers
    entity_field_list := '';
    entity_field_values := '';
    entity_update_list := '';
    entity_field_separator := '';

    -- Get columns from entity table, excluding uuid
    FOR column_record IN
        EXECUTE format('
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = ''public'' AND table_name = %L
            AND column_name <> ''uuid''
            ORDER BY ordinal_position
        ', table_name)
    LOOP
        column_name := column_record.column_name;

        -- For view column list
        IF entity_field_list <> '' THEN
            entity_field_list := entity_field_list || ', ';
        END IF;
        entity_field_list := entity_field_list || column_name;

        -- For update list
        IF entity_update_list <> '' THEN
            entity_update_list := entity_update_list || ', ';
        END IF;
        entity_update_list := entity_update_list || column_name || ' = NEW.' || column_name;
    END LOOP;

    -- Create view joining entity registry
    DECLARE
        view_query TEXT;
        column_list TEXT := '';
        registry_join TEXT;
    BEGIN
        -- Prepare column list for view
        IF entity_field_list <> '' THEN
            column_list := ', e.' || replace(entity_field_list, ', ', ', e.');
        END IF;

        registry_join := 'SELECT r.uuid, r.path, r.entity_key, r.created_at, r.updated_at, ' ||
                          'r.created_by, r.updated_by, r.published, r.version' ||
                          column_list ||
                          ' FROM entities_registry r ' ||
                          'LEFT JOIN ' || table_name || ' e ON r.uuid = e.uuid ' ||
                          'WHERE r.entity_type = ''' || entity_type_param || '''';

        view_query := 'CREATE VIEW ' || view_name || ' AS ' || registry_join;

        RAISE NOTICE 'Creating view with: %', view_query;
        EXECUTE view_query;

        -- Grant permissions
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %I TO PUBLIC', view_name);
    END;

    -- Create INSTEAD OF INSERT trigger - simple version
    trigger_name := view_name || '_insert_trigger';
    trigger_sql := '
        CREATE OR REPLACE FUNCTION ' || trigger_name || '()
        RETURNS TRIGGER AS $BODY$
        DECLARE
            new_uuid UUID;
        BEGIN
            -- Generate UUID if not provided
            IF NEW.uuid IS NULL THEN
                NEW.uuid := uuid_generate_v7();
            END IF;

            -- Set default values if not provided
            IF NEW.path IS NULL THEN
                NEW.path := ''/'';
            END IF;

            -- entity_key is NOT NULL on table; rely on constraint instead of manual check

            IF NEW.created_at IS NULL THEN
                NEW.created_at := NOW();
            END IF;

            IF NEW.updated_at IS NULL THEN
                NEW.updated_at := NOW();
            END IF;

            -- Insert into entities_registry
            INSERT INTO entities_registry (
                uuid, entity_type, path, entity_key, created_at, updated_at,
                created_by, updated_by, published, version
            )
            VALUES (
                NEW.uuid, ''' || entity_type_param || ''', NEW.path, NEW.entity_key, NEW.created_at, NEW.updated_at,
                NEW.created_by, NEW.updated_by, COALESCE(NEW.published, false), COALESCE(NEW.version, 1)
            )
            RETURNING uuid INTO new_uuid;';

    -- Add entity-specific insert if needed
    IF entity_field_list <> '' THEN
        trigger_sql := trigger_sql || '

            -- Insert into entity table with fields
            INSERT INTO ' || table_name || ' (uuid, ' || entity_field_list || ')
            VALUES (new_uuid';

        -- Add each field as a separate value
        FOR column_name IN
            SELECT unnest(string_to_array(entity_field_list, ', '))
        LOOP
            trigger_sql := trigger_sql || ', NEW.' || trim(column_name);
        END LOOP;

        trigger_sql := trigger_sql || ');';
    ELSE
        trigger_sql := trigger_sql || '

            -- Insert into entity table (UUID only)
            INSERT INTO ' || table_name || ' (uuid)
            VALUES (new_uuid);';
    END IF;

    -- Finish the trigger function
    trigger_sql := trigger_sql || '

            RETURN NEW;
        END;
        $BODY$ LANGUAGE plpgsql;';

    -- Create the function and trigger
    EXECUTE trigger_sql;

    EXECUTE 'DROP TRIGGER IF EXISTS ' || trigger_name || ' ON ' || view_name || ';';
    EXECUTE 'CREATE TRIGGER ' || trigger_name || '
             INSTEAD OF INSERT ON ' || view_name || '
             FOR EACH ROW EXECUTE FUNCTION ' || trigger_name || '();';

    -- Create INSTEAD OF UPDATE trigger - simple version
    trigger_name := view_name || '_update_trigger';
    trigger_sql := '
        CREATE OR REPLACE FUNCTION ' || trigger_name || '()
        RETURNS TRIGGER AS $BODY$
        BEGIN
            -- Update entities_registry
            UPDATE entities_registry
            SET path = NEW.path,
                entity_key = NEW.entity_key,
                updated_at = COALESCE(NEW.updated_at, NOW()),
                updated_by = NEW.updated_by,
                published = NEW.published,
                version = NEW.version
            WHERE uuid = NEW.uuid;';

    -- Add entity-specific update if we have fields
    IF entity_update_list <> '' THEN
        trigger_sql := trigger_sql || '

            -- Update entity table
            UPDATE ' || table_name || '
            SET ' || entity_update_list || '
            WHERE uuid = NEW.uuid;';
    END IF;

    -- Finish the trigger function
    trigger_sql := trigger_sql || '

            RETURN NEW;
        END;
        $BODY$ LANGUAGE plpgsql;';

    -- Create the function and trigger
    EXECUTE trigger_sql;

    EXECUTE 'DROP TRIGGER IF EXISTS ' || trigger_name || ' ON ' || view_name || ';';
    EXECUTE 'CREATE TRIGGER ' || trigger_name || '
             INSTEAD OF UPDATE ON ' || view_name || '
             FOR EACH ROW EXECUTE FUNCTION ' || trigger_name || '();';

    -- Create INSTEAD OF DELETE trigger - simple version
    trigger_name := view_name || '_delete_trigger';
    EXECUTE '
        CREATE OR REPLACE FUNCTION ' || trigger_name || '()
        RETURNS TRIGGER AS $BODY$
        BEGIN
            -- Delete from entities_registry (will cascade to entity table)
            DELETE FROM entities_registry
            WHERE uuid = OLD.uuid;

            RETURN OLD;
        END;
        $BODY$ LANGUAGE plpgsql;';

    EXECUTE 'DROP TRIGGER IF EXISTS ' || trigger_name || ' ON ' || view_name || ';';
    EXECUTE 'CREATE TRIGGER ' || trigger_name || '
             INSTEAD OF DELETE ON ' || view_name || '
             FOR EACH ROW EXECUTE FUNCTION ' || trigger_name || '();';

    RAISE NOTICE 'Successfully created/updated entity table and view for %', entity_type_param;
END;
$$ LANGUAGE plpgsql;

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
    recipient_uuid UUID NOT NULL REFERENCES admin_users(uuid) ON DELETE CASCADE,
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

-- Enums
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workflow_kind') THEN
CREATE TYPE workflow_kind AS ENUM ('consumer', 'provider');
END IF;
END $$;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workflow_run_status') THEN
CREATE TYPE workflow_run_status AS ENUM ('queued', 'running', 'success', 'failed', 'cancelled');
END IF;
END $$;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'data_raw_item_status') THEN
CREATE TYPE data_raw_item_status AS ENUM ('queued', 'processed', 'failed');
END IF;
END $$;

-- Workflows (definitions)
CREATE TABLE IF NOT EXISTS workflows (
     uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
     name VARCHAR(100) NOT NULL UNIQUE,
     description TEXT,
     kind workflow_kind NOT NULL,
     enabled BOOLEAN NOT NULL DEFAULT TRUE,
     schedule_cron TEXT,
     consumer_config JSONB,
     provider_config JSONB,
     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
     created_by UUID NOT NULL,
     updated_by UUID,
     version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_workflows_kind ON workflows(kind);
CREATE INDEX IF NOT EXISTS idx_workflows_enabled ON workflows(enabled);

-- Auto-update trigger for updated_at
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'set_timestamp_workflows'
    ) THEN
CREATE TRIGGER set_timestamp_workflows
    BEFORE UPDATE ON workflows
    FOR EACH ROW
    EXECUTE FUNCTION update_timestamp();
END IF;
END $$;

-- Workflow runs
CREATE TABLE IF NOT EXISTS workflow_runs (
     uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
     workflow_uuid UUID NOT NULL REFERENCES workflows(uuid) ON DELETE CASCADE,
     status workflow_run_status NOT NULL DEFAULT 'queued',
     trigger_id UUID,
     queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
     started_at TIMESTAMPTZ,
     finished_at TIMESTAMPTZ,
     total_items INTEGER NOT NULL DEFAULT 0,
     processed_items INTEGER NOT NULL DEFAULT 0,
     failed_items INTEGER NOT NULL DEFAULT 0,
     error TEXT
);

CREATE INDEX IF NOT EXISTS idx_workflow_runs_workflow_uuid ON workflow_runs(workflow_uuid);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_status ON workflow_runs(status);

-- Raw staged items per run
CREATE TABLE IF NOT EXISTS workflow_raw_items (
    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    workflow_run_uuid UUID NOT NULL REFERENCES workflow_runs(uuid) ON DELETE CASCADE,
    seq_no BIGINT NOT NULL,
    payload JSONB NOT NULL,
    status data_raw_item_status NOT NULL DEFAULT 'queued',
    error TEXT,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_workflow_raw_items_run_uuid ON workflow_raw_items(workflow_run_uuid);
CREATE INDEX IF NOT EXISTS idx_workflow_raw_items_status ON workflow_raw_items(status);
CREATE INDEX IF NOT EXISTS idx_workflow_raw_items_seq ON workflow_raw_items(workflow_run_uuid, seq_no);

-- Create a trigger to update entity views when entity definitions change
CREATE OR REPLACE FUNCTION entity_view_on_class_change()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM create_entity_table_and_view(NEW.entity_type);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Set up the trigger
DROP TRIGGER IF EXISTS trigger_create_entity_view ON entity_definitions;
CREATE TRIGGER trigger_create_entity_view
AFTER INSERT OR UPDATE ON entity_definitions
FOR EACH ROW
EXECUTE FUNCTION entity_view_on_class_change();

-- Create entity tables and views for existing entity definitions
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT entity_type FROM entity_definitions
    LOOP
        PERFORM create_entity_table_and_view(r.entity_type);
    END LOOP;
END $$;
