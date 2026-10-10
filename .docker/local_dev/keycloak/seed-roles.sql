-- Roles the Keycloak realm's groups map onto.
--
-- Run once against the dev database after starting the SSO stack:
--
--   docker compose exec -T postgres psql -U postgres -d rdata \
--     < .docker/local_dev/keycloak/seed-roles.sql
--
-- Nothing seeds roles in this project, and a role map naming a role that does
-- not exist has no effect — every identity then maps to nothing and is
-- refused, which looks like a broken guard rather than a missing fixture.
--
-- Neither role is `super_admin`, and that is deliberate rather than an
-- oversight: claim mapping filters super-admin roles out and logs the drop, so
-- a group mapped to one would be refused. Super-admin is granted inside
-- RDataCore, to a named person. See docs/SSO.md.

INSERT INTO roles (name, description, permissions, super_admin, created_by)
VALUES (
    'sso-admin',
    'Mapped from the rdc-admins group in Keycloak (local development)',
    '[
        {"resource_type": "Workflows",        "permission_type": "Admin",  "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "Entities",         "permission_type": "Admin",  "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "EntityDefinitions","permission_type": "Admin",  "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "ApiKeys",          "permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "Users",            "permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "Roles",            "permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "System",           "permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "DashboardStats",   "permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null}
    ]'::jsonb,
    false,
    '00000000-0000-0000-0000-000000000000'::uuid
)
ON CONFLICT (name) DO NOTHING;

INSERT INTO roles (name, description, permissions, super_admin, created_by)
VALUES (
    'sso-editor',
    'Mapped from the rdc-editors group in Keycloak (local development)',
    '[
        {"resource_type": "Workflows",        "permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "Workflows",        "permission_type": "Update", "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "Entities",         "permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "EntityDefinitions","permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null},
        {"resource_type": "DashboardStats",   "permission_type": "Read",   "access_level": "All", "resource_uuids": [], "constraints": null}
    ]'::jsonb,
    false,
    '00000000-0000-0000-0000-000000000000'::uuid
)
ON CONFLICT (name) DO NOTHING;

SELECT name, super_admin, jsonb_array_length(permissions) AS permission_count
FROM roles
WHERE name IN ('sso-admin', 'sso-editor');
