---
name: worker-crate
description: Background job worker, scheduled tasks, maintenance in crates/worker/
---

# Worker Crate (`r_data_core_worker`)

**Path**: `crates/worker/`
**Role**: Async background job processing and scheduled maintenance tasks.
**Depends on**: core, persistence, services, workflow

## Binaries

| Binary | Purpose |
|--------|---------|
| `r_data_core_worker` | Primary worker process (workflow job processing) |
| `r_data_core_maintenance` | Maintenance task runner |
| `send_statistics` | Statistics export tool |

## Key Modules

| Module | Responsibility |
|--------|----------------|
| `context.rs` | Worker execution context |
| `registrars/` | Task scheduler registrations |
| `tasks/` | Task implementations |

### Registrars (Schedule Definitions)

| File | Purpose |
|------|---------|
| `registrars/trait_.rs` | Registrar trait interface |
| `registrars/statistics.rs` | Statistics collection schedule |
| `registrars/refresh_token.rs` | Token cleanup schedule |
| `registrars/workflow_run_logs_purger.rs` | Log cleanup schedule |
| `registrars/version_purger.rs` | Version history cleanup schedule |
| `registrars/license.rs` | License verification schedule |

### Tasks (Implementations)

| File | Purpose |
|------|---------|
| `tasks/statistics_collection.rs` | Collect system statistics |
| `tasks/refresh_token_cleanup.rs` | Clean expired tokens |
| `tasks/workflow_run_logs_purger.rs` | Purge old workflow logs |
| `tasks/version_purger.rs` | Remove old entity versions |
| `tasks/license_verification.rs` | Verify license status |

## Scheduler

Uses `tokio_cron_scheduler` with `JobScheduler` for cron-based task scheduling.

## Patterns

- Registrar/Task separation: registrars define _when_, tasks define _what_
- Each scheduled task is independently configurable
- Worker context holds shared state (DB pool, cache, services)
