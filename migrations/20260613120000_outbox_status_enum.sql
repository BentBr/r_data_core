-- Convert outbox_messages.status from TEXT + CHECK to a dedicated Postgres enum,
-- matching the other status enums in the schema (workflow_run_status, etc.).

CREATE TYPE outbox_status AS ENUM ('pending', 'processing', 'delivered', 'retry', 'dead_letter');

-- Drop the objects that reference the status column before changing its type.
DROP INDEX IF EXISTS outbox_messages_status_available_at_idx;
DROP INDEX IF EXISTS outbox_messages_claim_due_idx;
ALTER TABLE outbox_messages DROP CONSTRAINT IF EXISTS outbox_messages_status_check;

-- Convert the column (the existing TEXT values already match the enum labels).
ALTER TABLE outbox_messages ALTER COLUMN status DROP DEFAULT;
ALTER TABLE outbox_messages
    ALTER COLUMN status TYPE outbox_status USING status::outbox_status;
ALTER TABLE outbox_messages ALTER COLUMN status SET DEFAULT 'pending';

-- Recreate the indexes (the enum enforces valid values, so the CHECK is dropped).
CREATE INDEX IF NOT EXISTS outbox_messages_status_available_at_idx
    ON outbox_messages (status, available_at);

CREATE INDEX IF NOT EXISTS outbox_messages_claim_due_idx
    ON outbox_messages (available_at, created_at)
    WHERE status IN ('pending', 'retry');
