# Docker Compose — Production-Flavored Example

> **⚠️ Example only — not production-ready as-is.** No warranty or guarantee. Review every line before deploying. You are responsible for secrets management, TLS termination, backups, and resource sizing.

This example deploys the full RDataCore backend (API, worker, maintenance, PostgreSQL, Redis) on a single host using pre-built images from GitHub Container Registry.

It differs from the in-repo development `compose.yaml` at the repo root:

- Uses pre-built images (no local build context)
- No `dinghy` / `VIRTUAL_HOST` routing — publishes on `localhost:8080`
- Omits dev-only services (mailpit, playwright, frontend node container, nginx-proxy, postgres_test)
- `restart: unless-stopped` on every service
- Secrets loaded from a user-created `.env`

## Prerequisites

- Docker Engine 20.10+ with Compose v2 plugin
- A valid RDataCore license key

## Setup

```bash
cd docs/examples/docker-compose
cp env.example .env
# Edit .env and fill in POSTGRES_PASSWORD, JWT_SECRET, LICENSE_KEY
```

> The template is named `env.example` (no leading dot) because dotfile templates are noisy in tooling. Your actual secrets file is still `.env`.

Generate strong secrets, for example:

```bash
openssl rand -base64 32   # Use for POSTGRES_PASSWORD and JWT_SECRET
```

## Run migrations (one-shot)

The `migrate` service is gated behind a Compose profile so it only runs when you ask for it:

```bash
docker compose --profile migrate run --rm migrate
```

This executes `/usr/local/bin/run_migrations` against the Postgres database and exits. Re-run this step after every upgrade that ships new migrations.

## Start the stack

```bash
docker compose up -d
```

The API becomes available at `http://localhost:8080`. Check health:

```bash
curl -sf http://localhost:8080/api/v1/health | jq
```

## Upgrade

```bash
docker compose pull
docker compose --profile migrate run --rm migrate
docker compose up -d
```

## Stop / tear down

```bash
docker compose down          # keeps volumes
docker compose down -v       # also drops postgres_data and redis_data — DESTRUCTIVE
```

## Caveats

- `:latest` image tag drifts. For any real deployment pin to a specific version tag.
- The API is exposed on `localhost:8080` without TLS. Put it behind a reverse proxy (Caddy, nginx, Traefik) terminating TLS.
- `CORS_ORIGINS=*` is permissive — restrict this to your frontend origin(s).
- No external SMTP is included. Set `SYSTEM_SMTP_DSN` / `WORKFLOW_SMTP_DSN` in `.env` to enable email.
- Postgres data lives in a named Docker volume. Back it up with your tool of choice (`pg_dump`, `pgBackRest`, Restic, etc.).
