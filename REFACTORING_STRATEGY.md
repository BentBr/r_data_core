use std::collections::HashMap;

# Refactoring Strategy for r_data_core

## Implementation Status

We've implemented several key phases of the refactoring strategy:

### API Key Refactoring
1. ✅ Created a repository trait for API keys (`ApiKeyRepositoryTrait`)
2. ✅ Implemented the trait for the existing repository (`ApiKeyRepository`)
3. ✅ Created a service layer for API keys (`ApiKeyService`)
4. ✅ Added unit tests for the service layer with mock repositories
5. ✅ Fixed issues with the API key service tests (lifetime problems, unused variables)
6. ✅ Set up an integration test structure with:
   - Common test utilities for database setup
   - Repository tests
   - API key service tests
7. ✅ Created a repository adapter pattern for adapter traits

### Admin User Refactoring
1. ✅ Created a repository trait for admin users (`AdminUserRepositoryTrait`)
2. ✅ Implemented the trait for the existing repository (`AdminUserRepository`)
3. ✅ Created a service layer for admin users (`AdminUserService`)
4. ✅ Added unit tests for the service layer with mock repositories
5. ✅ Set up an environment for tests:
   - Database utilities for testing repositories
   - Mocking library for testing services

### Dynamic Entity Refactoring
1. ✅ Created a repository trait for dynamic entities (`DynamicEntityRepositoryTrait`)
2. ✅ Implemented the trait for the existing repository (`DynamicEntityRepository`)
3. ✅ Created a service layer for dynamic entities (`DynamicEntityService`)
4. ✅ Added unit tests for the service layer with mock repositories
5. ✅ Set up a validation system for entity data validation
6. ✅ Added support for custom validations and hooks in dynamic entities
7. ✅ Implemented RESTful API for dynamic entities in the public API with custom namespaces

### File Structure Improvements
1. ✅ Split large files into smaller, more focused modules
   - Split `src/entity/field/definition.rs` (863 lines) into:
     - `src/entity/field/definition/mod.rs` - Main struct definition and exports
     - `src/entity/field/definition/schema.rs` - Schema conversion methods
     - `src/entity/field/definition/validation.rs` - Value validation logic
     - `src/entity/field/definition/constraints.rs` - Constraint handling and validation
     - `src/entity/field/definition/serialization.rs` - Serialization/deserialization logic

### Error Handling Improvements
1. ✅ Added basic error handling tests to ensure error messages are formatted properly

## Remaining Tasks

### File Structure Improvements
1. ⬜ Continue splitting large files:
   - `src/services/entity_definition_service.rs` (772 lines)
   - `src/entity/dynamic_entity/repository.rs` (683 lines)
   - `src/entity/dynamic_entity/validator.rs` (542 lines)
   - `src/api/admin/auth.rs` (470 lines)

### Error Handling Improvements
1. ⬜ Enhance the `Error` enum structure:
   - Add error codes to all error variants
   - Add context information to errors
   - Implement helper functions for error context and codes

2. ⬜ Improve error response handling:
   - Create standardized error responses with error codes
   - Add structured logging for errors
   - Implement proper HTTP status code mapping

3. ⬜ Create a middleware for error logging:
   - Log errors with context
   - Track error frequency and patterns

### API Standardization
1. ⬜ Create consistent API response formats:
   - Standardize success and error response formats
   - Implement common response envelope structure with:
     - Status code
     - Message
     - Data payload
     - Metadata (pagination, etc.)
   - Add support for pagination metadata
   - Implement field selection capabilities

2. ⬜ Implement standard query parameters:
   - Sorting parameters (`sort_by`, `sort_direction`)
   - Filtering parameters (`filter[]`, `q` for search)
   - Pagination parameters (`page`, `per_page` or `limit`, `offset`)
   - Field selection parameters (`fields[]`)
   - Include parameters for related data (`include[]`)

3. ⬜ Standardize endpoint naming and HTTP methods:
   - Use proper HTTP methods (GET, POST, PUT, DELETE, PATCH)
   - Follow RESTful resource naming conventions
   - Implement consistent path parameters
   - Use plural resource names for collections
   - Implement versioning strategy (URI, header, or content negotiation)

4. ⬜ Create documentation for API standards:
   - Document response formats
   - Document query parameter usage
   - Create examples for common operations
   - Implement OpenAPI/Swagger documentation

## Implementation Plan

### Near-term (Next 2 weeks)
1. Complete file structure refactoring
2. Implement error handling improvements
3. Begin API standardization

### Mid-term (Next 4-6 weeks)
1. Complete API standardization
2. Add comprehensive documentation
3. Create examples of API usage

### Long-term (Next 2-3 months)
1. Optimize performance
2. Add additional features
   - Enhanced caching
   - Batch operations
   - Real-time notifications

## Implementation Strategies

### Repository Pattern
Implement a clean repository pattern for all data access:
- Repository traits define the interface
- Concrete implementations provide the actual data access
- Services use repositories via traits (dependency injection)
- Mock repositories for testing

### Service Layer
Create a service layer for business logic:
- Services use repositories via traits
- Services expose high-level operations
- Services handle validation and business rules
- Services are testable with mock repositories

### API Layer
Standardize the API layer:
- Controllers use services for business logic
- Controllers handle HTTP concerns
- Controllers validate input
- Controllers format output
- Controllers handle errors

### Testing Strategy
Implement a comprehensive testing strategy:
- Unit tests for services with mock repositories
- Integration tests for repositories with test database
- Integration tests for API endpoints
- Performance tests for critical paths

## Design Principles

Throughout the refactoring, maintain these principles:

1. **Single Responsibility Principle**: Each module/class should have only one reason to change
2. **Dependency Inversion**: Depend on abstractions, not concrete implementations
3. **Interface Segregation**: Keep interfaces focused and specific
4. **Open/Closed Principle**: Open for extension, closed for modification
5. **Error Handling**: Provide clear, specific error messages that help debugging
6. **Validation**: Validate input early in the process flow
7. **Testing**: Write tests for both success and failure scenarios

By following this strategy, the codebase will become more maintainable, testable, and easier to extend.

## Best Practices Identified

1. **Repository Pattern**: Use repository interfaces (traits) to abstract database access
2. **Service Layer**: Implement business logic in services rather than directly in handlers
3. **Dependency Injection**: Use dependency injection for loosely coupled components
4. **Adapter Pattern**: Use adapters to implement traits for external types without modifying them
5. **Testing Isolation**: Use mock repositories for unit testing services
6. **Validation Flow**: Validate inputs before processing database operations to fail fast and prevent data corruption
7. **Edge Case Testing**: Test boundary conditions and error paths thoroughly
8. **API Contract Testing**: Ensure API endpoints adhere to the expected contract

## Repository Adapter Pattern

To handle the implementation of repository traits for existing repository implementations without modifying the original code or repeating boilerplate in main.rs, we've implemented an adapter pattern:

1. Created a `services/adapters.rs` module to house repository adapters
2. Each adapter wraps an actual repository and implements the corresponding trait
3. The adapter simply delegates method calls to the inner repository implementation
4. This allows for clean dependency injection without verbose code in main.rs
5. New repositories should follow this pattern when needed

Example:
```rust
// In services/adapters.rs
pub struct EntityDefinitionRepositoryAdapter {
    inner: EntityDefinitionRepository
}

#[async_trait]
impl EntityDefinitionRepositoryTrait for EntityDefinitionRepositoryAdapter {
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<EntityDefinition>> {
        self.inner.list(limit, offset).await
    }
    // Other methods delegated to inner repository
}

// In main.rs - clean usage
let adapter = EntityDefinitionRepositoryAdapter::new(repository);
let service = EntityDefinitionService::new(Arc::new(adapter));
```

This pattern keeps the codebase clean and maintainable as more repositories and services are added. 

## Edge Case Testing Strategy

For thorough testing, we now focus on these categories of edge cases:

1. **Data Validation**:
   - Empty/null inputs
   - Boundary values (min/max)
   - Malformed data
   - Type mismatches
   - Pattern violations (emails, URLs)
   - Duplicate data where uniqueness required

2. **Resource Handling**:
   - Not found scenarios
   - Permission/authorization failures
   - Rate limiting behavior
   - Concurrent access

3. **API Interactions**:
   - Malformed requests
   - Missing parameters
   - Invalid parameter types
   - Error response codes
   - Content negotiation

A detailed test plan has been created in TEST_PLAN.md to guide future testing efforts. 

## Implementation Plan

The implementation will proceed in phases:

1. **Phase 1: File Splitting** (3-4 weeks)
   - Split large files according to FILE_SPLITTING_PLAN.md
   - Update imports and exports
   - Ensure tests pass after each refactoring

2. **Phase 2: Error Handling** (2-3 weeks)
   - Implement new error structure
   - Add context builders
   - Update error conversions
   - Add structured logging

3. **Phase 3: API Standardization** (3-4 weeks)
   - Update response formats
   - Implement pagination, filtering, and sorting
   - Add field selection
   - Add request tracking

4. **Phase 4: Testing Improvements** (2-3 weeks)
   - Expand API tests
   - Add more integration tests
   - Add performance benchmarks
   - Document testing approaches 
