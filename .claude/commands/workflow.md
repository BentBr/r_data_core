# Workflow Management

Work with the workflow engine and DSL system.

## Architecture

Workflows use a DSL-based data pipeline system with:
- **FROM** step: Data source (URI, API, file upload)
- **TRANSFORM** step: Data transformation (field mapping, filtering)
- **TO** step: Destination (entity storage, HTTP endpoint, file)

Jobs are processed via two Redis queues using Apalis:
- Fetch queue: Retrieves data from sources
- Process queue: Transforms and stores data

## Key files

- `crates/workflow/src/dsl/` - DSL parsing and execution
- `crates/workflow/src/data/adapters/` - Source/destination adapters
- `crates/worker/src/` - Background job processing

## Testing workflows

```bash
# Run workflow-specific tests
cargo test workflow --workspace

# Run DSL tests
cargo test dsl --workspace

# Full e2e workflow tests
cargo test e2e_workflow --workspace
```

## API endpoints

### Admin API
- `POST /admin/api/v1/workflows` - Create workflow
- `PUT /admin/api/v1/workflows/{uuid}` - Update workflow
- `POST /admin/api/v1/workflows/{uuid}/run` - Trigger workflow
- `GET /admin/api/v1/workflows/{uuid}/runs` - List runs
- `POST /admin/api/v1/dsl/validate` - Validate DSL config

### Public API
- `POST /api/v1/workflows/{uuid}` - Ingest data (Consumer workflows)
- `GET /api/v1/workflows/{uuid}` - Get workflow data (Provider workflows)
- `GET /api/v1/workflows/{uuid}/trigger` - Trigger execution

## Example DSL

```json
{
  "from": {
    "type": "uri",
    "uri": "https://api.example.com/data",
    "format": "json"
  },
  "transform": {
    "field_mappings": [
      {"source": "$.name", "target": "name"},
      {"source": "$.value", "target": "amount"}
    ]
  },
  "to": {
    "type": "entity",
    "entity_type": "products"
  }
}
```
