## Todos
- add options for custom tables (like bricks)
- have easy-creation of children (entities)
- custom field type (json with predefined content - like a preferences structure...)
- key-value-store
- relations 1:n + n:n
- admin swagger field definitions / constraints
- load/performance test binary
- typescript bindings
- anyhow in the entire repo
- run now file upload with different file types / formats
- add unique constraint to entity_definitions (FE + BE)
- role testing and creation
    - entities: publish
    - workflows: activate
    - entity_definitions: publish
    - versions -> are included in read permissions
    - FE must react to those permissions
- notification / message system
    - messages
    - update on successfully run workflows
    - user requests permission(s)
    - default admin user not changed

Check DSL:

- map to validation

fixes:

- setting all fields for dynamic entities
- auth tests for all api routes
- tests (unit and integration) for dynamic entities (more)
- getting all entity types with fields and validations
- filter entities (by field and value) (validate against entity-definition)


- check manual ci deploy demo stuff
