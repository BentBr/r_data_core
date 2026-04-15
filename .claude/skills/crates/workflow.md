---
name: workflow-crate
description: Workflow DSL engine, job queue, data source/destination adapters in crates/workflow/
---

# Workflow Crate (`r_data_core_workflow`)

**Path**: `crates/workflow/`
**Role**: Self-contained DSL engine for data pipelines.
**Depends on**: core only

## Architecture

Workflows follow a strict three-step DSL: **from** -> **transform** -> **to**

Processing uses two queues via Apalis + Redis:
1. **Fetch queue** — retrieves data from sources
2. **Process queue** — transforms and writes to destinations

## Key Modules

| Module | Responsibility |
|--------|----------------|
| `dsl/` | DSL parsing, validation, and execution |
| `data/` | Data handling, job queues, source/destination adapters |

### DSL Module

| File | Purpose |
|------|---------|
| `dsl/from.rs` | Source definition: `EntityFilter`, `SourceConfig`, `FormatConfig` |
| `dsl/to.rs` | Target definition: `EntityWriteMode`, `OutputMode` |
| `dsl/transform.rs` | Transform types: Arithmetic, Concat, Authenticate |
| `dsl/path_resolution.rs` | Path resolution and field mapping |
| `dsl/execution.rs` | DSL execution engine |
| `dsl/validation.rs` | DSL validation |
| `dsl/program.rs` | `DslProgram` orchestration |

### Data Module

| File | Purpose |
|------|---------|
| `data/job_queue/apalis_redis.rs` | Redis-backed job queue (Apalis) |
| `data/jobs.rs` | Job definitions (`FetchAndStageJob`) |
| `data/requests.rs` | Request models |
| `data/adapters/source/` | Source configuration (URI, format) |
| `data/adapters/destination/` | Destination configuration |
| `data/adapters/auth.rs` | Authentication for external sources |
| `data/adapters/format/json.rs` | JSON format handler |
| `data/adapters/format/csv.rs` | CSV format handler |

## Key Exports

`DslProgram`, `DslStep`, `WorkflowKind`, `RunStatus`, transform types, source/destination configs, job queue interface

## Patterns

- Explicit DSL — no implicit behavior
- Async job processing via Apalis + Redis
- Format-agnostic adapters (JSON, CSV)
- Authentication support for external data sources (basic, bearer, custom)
