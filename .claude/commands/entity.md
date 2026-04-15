# Entity Management

Work with dynamic entities and entity definitions.

## Concepts

- **Entity Definition**: Schema that defines an entity type (fields, validation, UI settings)
- **Dynamic Entity**: Runtime instance of an entity type with JSONB fields
- **Auto-views**: Each entity type gets `entity_{type}` table and `entity_{type}_view`

## Key files

- `crates/core/src/entity_definition/` - Entity definition models
- `crates/core/src/field/` - Field types and definitions
- `crates/services/src/dynamic_entity/` - Entity CRUD operations
- `crates/services/src/entity_definition/` - Definition management
- `crates/persistence/src/dynamic_entity_repository*.rs` - Database access

## Field types

Supported field types:
- `String`, `Text`, `Integer`, `Float`, `Boolean`
- `Date`, `DateTime`, `Time`
- `Uuid`, `Email`, `Url`
- `Json`, `Array`
- `Reference` (foreign key to another entity)

## Testing entities

```bash
# Entity-related tests
cargo test entity --workspace
cargo test dynamic_entity --workspace
cargo test entity_definition --workspace
```

## API endpoints

### Admin API (Entity Definitions)
- `GET /admin/api/v1/entity-definitions` - List definitions
- `POST /admin/api/v1/entity-definitions` - Create definition
- `POST /admin/api/v1/entity-definitions/apply-schema` - Apply DB schema

### Public API (Entities)
- `GET /api/v1/{entity_type}` - List entities
- `POST /api/v1/{entity_type}` - Create entity
- `GET /api/v1/{entity_type}/{uuid}` - Get entity
- `PUT /api/v1/{entity_type}/{uuid}` - Update entity
- `DELETE /api/v1/{entity_type}/{uuid}` - Delete entity
- `POST /api/v1/{entity_type}/query` - Advanced query

## Versioning

Entity changes are versioned. Access versions via:
- `GET /api/v1/entities/{entity_type}/{uuid}/versions`
- `GET /api/v1/entities/{entity_type}/{uuid}/versions/{version}`
