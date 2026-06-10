---
name: api-reference
description: Public and Admin API endpoint reference with authentication details
---

# API Reference

OpenAPI specs:
- Public: `http://rdatacore.docker/api/docs/openapi.json`
- Admin: `http://rdatacore.docker/admin/api/docs/openapi.json`

## API Structure

- `/api/` — Public API (JWT or API key auth) for entity CRUD operations
- `/admin/api/` — Admin API (admin JWT only) for system configuration
- Swagger docs at `/api/docs/` and `/admin/api/docs/`

## Public API (`/api/v1/`)

Authentication: JWT (`Authorization: Bearer <token>`) or API Key (`X-API-Key: <key>`)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/entities` | List available entity types |
| GET | `/entities/by-path` | Browse entities by virtual folder path |
| GET | `/{entity_type}` | List entities with pagination/filtering |
| POST | `/{entity_type}` | Create entity |
| GET | `/{entity_type}/{uuid}` | Get entity by UUID |
| PUT | `/{entity_type}/{uuid}` | Update entity |
| DELETE | `/{entity_type}/{uuid}` | Delete entity |
| POST | `/{entity_type}/query` | Advanced query with complex filtering |
| GET | `/entities/{entity_type}/{uuid}/versions` | List entity versions |
| GET | `/entities/{entity_type}/{uuid}/versions/{version}` | Get specific version |
| GET | `/workflows/{uuid}` | Get workflow data (Provider) |
| POST | `/workflows/{uuid}` | Ingest data (Consumer with API source) |
| GET | `/workflows/{uuid}/stats` | Get workflow metadata |
| GET | `/workflows/{uuid}/trigger` | Trigger workflow execution |

## Admin API (`/admin/api/v1/`)

Authentication: Admin JWT only (`Authorization: Bearer <token>`)

### Authentication
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/auth/login` | Admin login |
| POST | `/auth/logout` | Logout (revoke refresh token) |
| POST | `/auth/refresh` | Refresh access token |
| POST | `/auth/register` | Register new admin user |
| POST | `/auth/revoke-all` | Revoke all refresh tokens |
| GET | `/auth/permissions` | Get user's permissions |

### Entity Definitions
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/entity-definitions` | List definitions |
| POST | `/entity-definitions` | Create definition |
| GET | `/entity-definitions/{uuid}` | Get definition |
| PUT | `/entity-definitions/{uuid}` | Update definition |
| DELETE | `/entity-definitions/{uuid}` | Delete definition |
| POST | `/entity-definitions/apply-schema` | Apply DB schema |
| GET | `/entity-definitions/{uuid}/versions` | List versions |

### Users & Roles
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST | `/users` | List/create users |
| GET/PUT/DELETE | `/users/{uuid}` | User CRUD |
| GET/PUT | `/users/{uuid}/roles` | User role assignment |
| GET/POST | `/roles` | List/create roles |
| GET/PUT/DELETE | `/roles/{uuid}` | Role CRUD |

### API Keys
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST | `/api-keys` | List/create API keys |
| DELETE | `/api-keys/{uuid}` | Revoke API key |

### Workflows
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST | `/workflows` | List/create workflows |
| GET/PUT/DELETE | `/workflows/{uuid}` | Workflow CRUD |
| POST | `/workflows/{uuid}/run` | Trigger workflow now |
| POST | `/workflows/{uuid}/run/upload` | Upload file for workflow run |
| GET | `/workflows/{uuid}/runs` | List workflow runs |
| GET | `/workflows/{uuid}/versions` | List workflow versions |
| GET | `/workflow-runs/{run_uuid}/logs` | Get run logs |
| GET | `/workflows/cron/preview` | Preview cron schedule |

### DSL Validation
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/dsl/validate` | Validate DSL config |
| GET | `/dsl/from/options` | FROM step options |
| GET | `/dsl/to/options` | TO step options |
| GET | `/dsl/transform/options` | TRANSFORM step options |

### System
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Admin health check |
| GET | `/meta/dashboard` | Dashboard statistics |
| GET/PUT | `/system/settings/entity-versioning` | Versioning settings |
| GET | `/system/license` | License status |
