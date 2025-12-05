-- Ensure workflows.created_by is NOT NULL and references admin_users(uuid)
ALTER TABLE workflows
    ALTER COLUMN created_by SET NOT NULL;

-- Add FK constraint if it does not exist
-- Check in current schema to support per-test schema isolation
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint c
        JOIN pg_class t ON c.conrelid = t.oid
        JOIN pg_namespace n ON t.relnamespace = n.oid
        WHERE c.conname = 'fk_workflows_created_by'
        AND n.nspname = current_schema()
        AND t.relname = 'workflows'
    ) THEN
        ALTER TABLE workflows
            ADD CONSTRAINT fk_workflows_created_by
            FOREIGN KEY (created_by)
            REFERENCES admin_users(uuid)
            ON DELETE RESTRICT;
    END IF;
END
$$;

-- Helpful index for filtering by creator
CREATE INDEX IF NOT EXISTS idx_workflows_created_by ON workflows(created_by);

-- Add FK constraint for updated_by (nullable)
-- Check in current schema to support per-test schema isolation
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint c
        JOIN pg_class t ON c.conrelid = t.oid
        JOIN pg_namespace n ON t.relnamespace = n.oid
        WHERE c.conname = 'fk_workflows_updated_by'
        AND n.nspname = current_schema()
        AND t.relname = 'workflows'
    ) THEN
        ALTER TABLE workflows
            ADD CONSTRAINT fk_workflows_updated_by
            FOREIGN KEY (updated_by)
            REFERENCES admin_users(uuid)
            ON DELETE RESTRICT;
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_workflows_updated_by ON workflows(updated_by);


