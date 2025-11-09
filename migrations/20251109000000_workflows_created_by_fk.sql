-- Ensure workflows.created_by is NOT NULL and references admin_users(uuid)
ALTER TABLE workflows
    ALTER COLUMN created_by SET NOT NULL;

-- Add FK constraint if it does not exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_workflows_created_by'
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
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_workflows_updated_by'
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


