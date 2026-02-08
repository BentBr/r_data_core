-- Seed default workflow_run_logs settings
INSERT INTO system_settings (key, value)
VALUES (
    'workflow_run_logs',
    jsonb_build_object(
        'enabled', true,
        'max_runs', null,
        'max_age_days', 90
    )
)
ON CONFLICT (key) DO NOTHING;
