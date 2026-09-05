-- Removes all E2E test data (e2e_* prefixed) from the local database
DO $$
DECLARE
    r RECORD;
BEGIN
    -- Drop dynamic entity tables and views for e2e entity definitions
    FOR r IN SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename LIKE 'entity_e2e_%' LOOP
        EXECUTE 'DROP TABLE IF EXISTS ' || quote_ident(r.tablename) || ' CASCADE';
        RAISE NOTICE 'Dropped table %', r.tablename;
    END LOOP;
    FOR r IN SELECT viewname FROM pg_views WHERE schemaname = 'public' AND viewname LIKE 'entity_e2e_%' LOOP
        EXECUTE 'DROP VIEW IF EXISTS ' || quote_ident(r.viewname) || ' CASCADE';
        RAISE NOTICE 'Dropped view %', r.viewname;
    END LOOP;

    -- Delete entities from registry that belong to e2e entity types
    DELETE FROM entities_registry WHERE entity_type LIKE 'e2e_%';

    -- Delete entity definition versions and definitions
    DELETE FROM entity_definition_versions WHERE definition_uuid IN (SELECT uuid FROM entity_definitions WHERE entity_type LIKE 'e2e_%');
    DELETE FROM entity_definitions WHERE entity_type LIKE 'e2e_%';

    -- Delete workflow data (runs, logs, versions, raw items cascade via FK)
    DELETE FROM workflows WHERE name LIKE 'e2e_%';

    -- Delete API keys (api_key_roles cascades via FK)
    DELETE FROM api_keys WHERE name LIKE 'e2e_%';

    -- Delete admin users (refresh_tokens, user_roles, notifications cascade via FK)
    DELETE FROM admin_users WHERE username LIKE 'e2e_%';

    -- Delete roles (user_roles, api_key_roles cascade via FK)
    DELETE FROM roles WHERE name LIKE 'e2e_%';
END $$;
