-- Component versions table for tracking worker/maintenance versions
-- This allows the API to report versions of all components

CREATE TABLE IF NOT EXISTS component_versions (
    component_name VARCHAR(64) PRIMARY KEY,
    version VARCHAR(32) NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying by last_seen_at (to detect stale components)
CREATE INDEX IF NOT EXISTS idx_component_versions_last_seen
    ON component_versions(last_seen_at);

COMMENT ON TABLE component_versions IS 'Tracks version information for distributed components (worker, maintenance)';
COMMENT ON COLUMN component_versions.component_name IS 'Unique name of the component (e.g., worker, maintenance)';
COMMENT ON COLUMN component_versions.version IS 'Semantic version of the component';
COMMENT ON COLUMN component_versions.last_seen_at IS 'Last time this component reported its version';
