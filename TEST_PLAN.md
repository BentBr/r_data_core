# Test Plan for r_data_core

This document outlines the testing strategy for the r_data_core project, with a focus on comprehensive test coverage and edge case handling.

## Testing Strategy

Our testing strategy follows a layered approach:

### 1. Unit Tests

Unit tests verify individual components in isolation with mocked dependencies.

**Focus areas:**
- Business logic validation
- Error handling
- Edge cases 
- Individual service methods

**Key traits:**
- Fast execution
- No database dependencies
- Complete coverage of business logic
- Isolated from other components

**Progress:**
- ✅ Services layer unit tests with mock repositories
- ✅ Basic error handling tests
- ⬜ Validation logic tests for all constraints

### 2. Integration Tests

Integration tests verify how components interact with each other, using real implementations.

**Focus areas:**
- Repository interactions with the database
- Service interactions with repositories
- Transaction handling
- Performance considerations

**Key traits:**
- Use test database
- Test real database queries
- Verify data persistence
- Test transactions and rollbacks

**Progress:**
- ✅ Basic repository integration tests
- ✅ Database setup utilities
- ⬜ Transaction and rollback tests
- ⬜ Concurrent access tests

### 3. API Tests

API tests verify HTTP endpoints and ensure proper request/response handling.

**Focus areas:**
- Route handling
- Request validation
- Response formatting
- Authentication and authorization
- Status code handling

**Key traits:**
- Test HTTP interactions
- Validate request parameters
- Verify response structures
- Test error responses

**Progress:**
- ✅ Basic CRUD endpoint tests
- ✅ Dynamic entity API tests
- ⬜ Authentication middleware tests
- ⬜ Error handling tests

## Test Coverage Goals

We aim for the following test coverage targets:

| Layer | Coverage Target | Current Status |
|-------|-----------------|----------------|
| Unit Tests | 85%+ | 65% (in progress) |
| Integration Tests | 70%+ | 45% (in progress) |
| API Tests | 80%+ | 50% (in progress) |

## Edge Case Testing

Each component should include tests for these common edge cases:

### Data Validation Edge Cases

- **Empty input**: Test with empty strings, empty collections, null values
- **Boundary values**: Test min/max values, just inside/outside valid ranges
- **Type mismatches**: Test with wrong data types
- **Pattern validation**: Test invalid patterns (email, URLs, etc.)
- **Duplicate data**: Test with duplicate values where uniqueness is required
- **Size limits**: Test with data that exceeds size limits

### Resource Edge Cases

- **Resource not found**: Test behavior when items don't exist
- **Permission denied**: Test with insufficient permissions
- **Rate limiting**: Test behavior at rate limits
- **Concurrent access**: Test with multiple simultaneous operations
- **Database connection issues**: Test behavior during connection failures
- **Timeout handling**: Test behavior when operations time out

### API Edge Cases

- **Malformed requests**: Test with invalid JSON, wrong content types
- **Missing parameters**: Test with missing required parameters
- **Invalid parameters**: Test with invalid parameter types/values
- **URL parameter handling**: Test with special characters, encoding issues
- **Large payloads**: Test with unusually large request bodies

## Priority Test Areas

Based on the refactoring strategy document, our immediate focus areas are:

1. **Dynamic Entity Service**
   - ✅ Entity validation against class definitions
   - ✅ Field type validation and constraints
   - ✅ Required field validation
   - ⬜ Pattern validation for string fields
   - ⬜ Range validation for numeric fields
   - ⬜ Enum value validation

2. **Public API Endpoints**
   - ✅ Dynamic entity CRUD operations
   - ⬜ Filter operations with complex criteria
   - ⬜ API key authentication
   - ⬜ Error handling and status codes

3. **Class Definition Service**
   - ✅ Entity type uniqueness validation
   - ✅ Field definition validation
   - ⬜ Schema updates and migrations

4. **Error Handling**
   - ✅ Basic error message formatting
   - ⬜ Error code standardization
   - ⬜ Error context propagation
   - ⬜ API error response format

## Integration Test Setup

All integration tests should:

1. Use a dedicated test database
2. Set up required test data
3. Clean up after tests complete
4. Use transactions where appropriate to isolate tests
5. Avoid dependencies between tests

## Mock Implementation Guidelines

When creating mocks for testing:

1. Use mockall for creating mock implementations
2. Mock at the trait boundary
3. Set up specific expectations for each test
4. Test both success and error paths
5. Implement minimal mock functionality to support the test

## Continuous Testing

Tests should be:

1. Automatic: Run via CI/CD pipeline on every commit
2. Fast: Optimize for quick execution
3. Reliable: Eliminate flaky tests
4. Isolated: No dependencies between tests
5. Self-checking: Tests should determine pass/fail automatically

## Test Naming Conventions

Tests should follow this naming pattern:

```
test_<method_name>_<scenario>
```

Examples:
- `test_create_entity_success`
- `test_create_entity_missing_required_field`
- `test_update_entity_validation_error`

## Implementation Plan

### Phase 1: Complete Service Layer Tests (Current Focus)

- ✅ Unit tests for dynamic entity service methods
- ✅ Unit tests for class definition service
- ✅ Unit tests for API key service
- ✅ Unit tests for admin user service
- ⬜ Edge case tests for validation logic
- ⬜ Error handling tests with context information

### Phase 2: Complete API Test Coverage

- ⬜ Tests for API standardization
- ⬜ Tests for authentication middleware
- ⬜ Tests for error handling middleware
- ⬜ Load/performance tests for critical endpoints

### Phase 3: Improve Repository Test Coverage

- ⬜ More comprehensive tests for all repositories
- ⬜ Test database concurrency scenarios
- ⬜ Test transaction rollback scenarios
- ⬜ Test connection error handling

### Phase 4: End-to-End Workflow Tests

- ⬜ Tests that cover complete business workflows
- ⬜ Test multi-step processes
- ⬜ Test full system integration 