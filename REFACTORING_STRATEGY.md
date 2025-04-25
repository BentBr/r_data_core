# Refactoring Strategy for r_data_core

## Implementation Status

We've implemented the first phase of the refactoring strategy, focusing on the API key and admin user functionality:

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

### Admin User Refactoring
1. ✅ Created a repository trait for admin users (`AdminUserRepositoryTrait`)
2. ✅ Implemented the trait for the existing repository (`AdminUserRepository`)
3. ✅ Created a service layer for admin users (`AdminUserService`)
4. ✅ Added unit tests for the service layer with mock repositories
5. ✅ Added integration tests for the admin user repository
6. ✅ Added JWT login test for admin users

### API Authentication
1. ✅ Fixed API key hashing and validation in authentication flow
2. ✅ Ensured API key last_used_at is updated correctly
3. ✅ Ensured JWT-based admin authentication works correctly
4. ✅ Added tests for both authentication paths

### Next Entity to Refactor: Class Definitions
1. 🔄 Create a repository trait for class definitions (`ClassDefinitionRepositoryTrait`)
2. 🔄 Implement the trait for the existing repository
3. 🔄 Create a service layer for class definitions (`ClassDefinitionService`)
4. 🔄 Add unit tests for the service layer with mock repositories
5. 🔄 Add integration tests for the class definition repository

## Next Steps

To continue the refactoring:

1. ✅ Create repository traits and services for key entities:
   - ✅ API Keys
   - ✅ Admin Users
   - 🔄 Class Definitions
   - ❌ Dynamic Entities
   - ❌ Workflows

2. 🔄 Update API handlers to use the service layer instead of repositories directly:
   - ✅ Inject the `ApiKeyService` into the routes
   - 🔄 Inject the `AdminUserService` into the admin routes
   - ❌ Update other API handlers to use appropriate services
   - ❌ Remove direct database access from handlers

3. ❌ Extract configuration management:
   - ❌ Create a unified config module
   - ❌ Use dependency injection for configuration

4. ❌ Split large files:
   - ❌ Break down large entity files into smaller components
   - ❌ Focus on cohesive modules with single responsibilities

5. 🔄 Complete test coverage:
   - ✅ Add repository tests for API keys
   - ✅ Add repository tests for admin users
   - ✅ Add service tests for API keys
   - ✅ Add service tests for admin users
   - 🔄 Add integration tests for all API endpoints
   - 🔄 Add repository tests for all database operations
   - 🔄 Add service tests for all business logic

## Current Issues Resolved

1. ✅ Fixed API key authentication by ensuring the key is hashed before querying
2. ✅ Fixed integration tests for API key service
3. ✅ Resolved conflicts with the `class_definition.rs` module
4. ✅ Improved error handling in authentication flows

## Testing Strategy

Three levels of testing:

1. **Unit Tests**:
   - Test individual components in isolation
   - Use mocks for dependencies
   - Focus on logic and edge cases
   - Negative test cases included for validation and error handling

2. **Integration Tests**:
   - Test repositories against real database
   - Test services with real repositories
   - Focus on interaction between components
   - Include potential failure scenarios

3. **API Tests**:
   - Test HTTP endpoints
   - Ensure correct request/response handling
   - Focus on API contract adherence
   - Include authentication/authorization scenarios
   - Test error responses and status codes

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