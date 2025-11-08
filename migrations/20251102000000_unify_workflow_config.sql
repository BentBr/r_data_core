-- Unify workflow configs into single JSONB `config`
ALTER TABLE workflows
    ADD COLUMN IF NOT EXISTS config JSONB NOT NULL DEFAULT '{}'::jsonb;

-- Migrate existing data from consumer/provider configs if present
UPDATE workflows
SET config = COALESCE(consumer_config, provider_config, '{}'::jsonb)
WHERE (config = '{}'::jsonb OR config IS NULL);

-- Drop old columns if they exist
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'workflows' AND column_name = 'consumer_config'
    ) THEN
        ALTER TABLE workflows DROP COLUMN consumer_config;
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'workflows' AND column_name = 'provider_config'
    ) THEN
        ALTER TABLE workflows DROP COLUMN provider_config;
    END IF;
END $$;

-- Create table for workflow run logs
CREATE TABLE IF NOT EXISTS workflow_run_logs (
    uuid UUID PRIMARY KEY DEFAULT uuidv7(),
    run_uuid UUID NOT NULL REFERENCES workflow_runs(uuid) ON DELETE CASCADE,
    ts TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    meta JSONB
);

CREATE INDEX IF NOT EXISTS idx_workflow_run_logs_run_uuid_ts
    ON workflow_run_logs (run_uuid, ts DESC);
