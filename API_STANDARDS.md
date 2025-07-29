# API Standards

This document outlines the standards for the r_data_core API, including endpoint naming, request/response formats, and common patterns.

## API Response Format

All API responses follow a consistent format:

```json
{
    "status": "Success|Error",
    "message": "Human-readable message",
    "data": <response data (optional)>,
    "meta": {
        "pagination": {
            "total": <total items>,
            "page": <current page>,
            "per_page": <items per page>,
            "total_pages": <total pages>,
            "has_previous": <boolean>,
            "has_next": <boolean>
        },
        "request_id": <UUID for request tracking>,
        "timestamp": <ISO 8601 timestamp>,
        "custom": <additional metadata>
    }
}
```

### Success Responses

For successful operations, the `status` field is set to `"Success"`, and the `data` field contains the response data.

Example:

```json
{
    "status": "Success",
    "message": "Operation completed successfully",
    "data": {
        "uuid": "123e4567-e89b-12d3-a456-426614174000",
        "name": "Example Entity"
    },
    "meta": {
        "request_id": "456e4567-e89b-12d3-a456-426614174001",
        "timestamp": "2023-06-01T12:34:56Z"
    }
}
```

### Error Responses

For operations that result in errors, the `status` field is set to `"Error"`, the `message` field contains a human-readable error message, and the `meta.custom` field contains an `error_code`.

Example:

```json
{
    "status": "Error",
    "message": "Entity with ID '123' not found",
    "data": null,
    "meta": {
        "request_id": "456e4567-e89b-12d3-a456-426614174001",
        "timestamp": "2023-06-01T12:34:56Z",
        "custom": {
            "error_code": "RESOURCE_NOT_FOUND"
        }
    }
}
```

### Paginated Responses

For endpoints that return collections of items, pagination information is included in the `meta.pagination` field.

Example:

```json
{
    "status": "Success",
    "message": "Operation completed successfully",
    "data": [
        { "uuid": "123e4567-e89b-12d3-a456-426614174000", "name": "Item 1" },
        { "uuid": "123e4567-e89b-12d3-a456-426614174001", "name": "Item 2" }
    ],
    "meta": {
        "pagination": {
            "total": 100,
            "page": 1,
            "per_page": 10,
            "total_pages": 10,
            "has_previous": false,
            "has_next": true
        },
        "request_id": "456e4567-e89b-12d3-a456-426614174001",
        "timestamp": "2023-06-01T12:34:56Z"
    }
}
```

## Query Parameters

The API supports the following standard query parameters:

### Pagination

- `page`: The page number (1-based)
- `per_page`: The number of items per page (default: 20, max: 100)

Example:
```
GET /api/v1/users?page=2&per_page=50
```

### Sorting

- `sort_by`: The field to sort by
- `sort_direction`: The sort direction (`ASC` or `DESC`)

Example:
```
GET /api/v1/users?sort_by=created_at&sort_direction=DESC
```

### Field Selection

- `fields`: Comma-separated list of fields to include

Example:
```
GET /api/v1/users?fields=uuid,username,email
```

### Filtering

- `filter`: Filter expression (JSON format or compact format)
- `q`: Search query

Example with JSON format:
```
GET /api/v1/users?filter={"status":"active","role":"admin"}
```

Example with compact format:
```
GET /api/v1/users?filter=status:active,role:admin
```

Example with search:
```
GET /api/v1/users?q=john
```

### Including Related Resources

- `include`: Comma-separated list of related resources to include

Example:
```
GET /api/v1/users?include=roles,permissions
```

## HTTP Methods

The API uses the following HTTP methods:

- `GET`: Retrieve resources
- `POST`: Create resources
- `PUT`: Update resources (full replacement)
- `PATCH`: Update resources (partial update)
- `DELETE`: Delete resources

## URL Structure

The API uses the following URL structure:

- Resource collections: `/api/v1/{resource}`
- Specific resource: `/api/v1/{resource}/{id}`
- Related resource collections: `/api/v1/{resource}/{id}/{related-resource}`
- Actions on resources: `/api/v1/{resource}/{id}/{action}`

Examples:
```
GET /api/v1/users                  # List users
GET /api/v1/users/123              # Get a specific user
GET /api/v1/users/123/permissions  # List permissions for a user
POST /api/v1/users/123/activate    # Activate a user
```

## Error Codes

The API uses the following standard error codes:

| HTTP Status | Error Code | Description |
|-------------|------------|-------------|
| 400 | BAD_REQUEST | The request was malformed or invalid |
| 401 | UNAUTHORIZED | Authentication is required |
| 403 | FORBIDDEN | The authenticated user lacks permission |
| 404 | RESOURCE_NOT_FOUND | The requested resource was not found |
| 409 | RESOURCE_CONFLICT | The request conflicts with the current state |
| 422 | VALIDATION_ERROR | The request contains invalid data |
| 500 | INTERNAL_SERVER_ERROR | An unexpected error occurred |

## Error Handling Middleware

The API now includes an error handling middleware (`ErrorHandler`) that ensures all errors follow our standardized response format. The middleware:

1. Intercepts all error responses
2. Formats them according to our API response standards
3. Adds appropriate error codes and metadata
4. Returns consistent JSON structures for all errors

This ensures that all API responses, even in error cases, maintain a consistent format for clients.

## Versioning

The API is versioned in the URL path: `/api/v1/`.

## Authentication

The API supports the following authentication methods:

1. JWT Bearer Token:
```
Authorization: Bearer <token>
```

2. API Key:
```
X-API-Key: <api-key>
``` 