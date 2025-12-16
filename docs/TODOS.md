## Todos
- add options for custom tables (like bricks)
- have easy-creation of children (entities)
- test admin routes
- check entities and respective columns - we need proper creation and not everything serialized.
- check env vars
- custom field type (json with predefined content - like a preferences structure...)
- key-value-store
- relations 1:n + n:n
- admin swagger field definitions / constraints
- crons + refresh token deletion
- load/performance test binary
- typescript bindings
- anyhow in the entire repo
- run now file upload with different file types / formats
- DSL FE + be tests
- DSL readme
- admin easy default password warning (admin admin) -> hint in admin if default pw is being used
- add unique constraint to entity_definitions (FE + BE)
- uuid refactoring -> all in db
- toDef ->update action -> by key -> dropdown of existing one
- push all path / params / post body usages to dependency injection
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

- map to entity
- map to field
- map to validation
- map to uri / source
- calculate (basic arithmetic)
- concatenation of string (conversion of int, float etc)

fixes:

- setting all fields for dynamic entities
- auth tests for all api routes
- tests (unit and integration) for dynamic entities (more)
- getting all entity types with fields and validations
- filter entities (by field and value) (validate against entity-definition)


- Release
  - Register domain rdatacore.eu
    - emailing
  - setup subdomain: demo.rdatacore.eu
  - find a hoster -> deploy website (statically)
    - check sitemap + robots.txt for google
    - setup search console account
  - find a hoster -> deploy RDataCore FE + Backend (demo)
- communicate
  - LinkedIn groups
  - Stefano
  - Matthias
  - Jan B.
- Check all provided images (ghcr.io)
  
