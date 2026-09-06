#!/bin/sh
# Seed the RDataCore roles that the Keycloak realm's groups map onto.
#
# Runs as a one-shot compose service after migrations. Idempotent: the SQL uses
# ON CONFLICT DO NOTHING, so repeated `compose up` is harmless.
#
# It waits for the `roles` table rather than assuming it. Migrations are a
# separate service, and on a cold volume this can start first; failing here
# with "relation does not exist" would be a confusing way to learn that.

set -eu

ATTEMPTS=60
SLEEP=2

echo "sso-seed: waiting for the roles table…"
i=0
while [ "$i" -lt "$ATTEMPTS" ]; do
    if psql -tAc "SELECT to_regclass('public.roles');" 2>/dev/null | grep -q '^roles$'; then
        echo "sso-seed: schema is ready"
        psql -v ON_ERROR_STOP=1 -f /seed/seed-roles.sql
        echo "sso-seed: done"
        exit 0
    fi
    i=$((i + 1))
    sleep "$SLEEP"
done

echo "sso-seed: the roles table never appeared after $((ATTEMPTS * SLEEP))s." >&2
echo "sso-seed: migrations may have failed — check the sso-migrate service." >&2
exit 1
