CREATE TABLE IF NOT EXISTS outbox_messages (
    uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    topic TEXT NOT NULL,
    kind TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'pending',
    attempt_count INTEGER NOT NULL DEFAULT 0,
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_at TIMESTAMPTZ,
    locked_by TEXT,
    last_error TEXT,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    CONSTRAINT outbox_messages_status_check CHECK (status IN ('pending', 'processing', 'delivered', 'retry', 'dead_letter'))
);

CREATE UNIQUE INDEX IF NOT EXISTS outbox_messages_idempotency_key_idx
    ON outbox_messages (idempotency_key);

CREATE INDEX IF NOT EXISTS outbox_messages_status_available_at_idx
    ON outbox_messages (status, available_at);

CREATE INDEX IF NOT EXISTS outbox_messages_aggregate_idx
    ON outbox_messages (aggregate_type, aggregate_id);
