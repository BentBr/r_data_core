# DSL (Domain Specific Language) Documentation

## Overview

The DSL (Domain Specific Language) is a powerful workflow configuration system that allows you to define data transformations through a series of steps. Each step can read from various sources, transform data, and output to different destinations.

## Key Concepts

### Steps

A DSL program consists of one or more **steps**. Each step has three components:

1. **From** (`from`): Defines where data comes from
2. **Transform** (`transform`): Defines what transformations to apply
3. **To** (`to`): Defines where data goes

### Step Chaining

Steps can be chained together, where the output of one step becomes the input for the next:

- **PreviousStep** (`FromDef`): Read data from the previous step's output
- **NextStep** (`ToDef`): Explicitly pass data to the next step (with optional field mapping)

### Normalized Data

Within each step, data is normalized into a consistent JSON structure. This normalized data is:
- Used for transformations (arithmetic, concatenation)
- Available for mapping to output formats
- Passed between steps when chaining

## FromDef Types

### Format

Read data from CSV, JSON, or other formats:

```json
{
  "type": "format",
  "source": {
    "source_type": "api",
    "config": {}
  },
  "format": {
    "format_type": "json",
    "options": {}
  },
  "mapping": {
    "source_field": "normalized_field"
  }
}
```

**Source Types:**
- **API** (`source_type: "api"`): Accepts POST data via `/api/v1/workflows/{uuid}` endpoint. Used for webhook ingestion with data payload.
- **URI** (`source_type: "uri"`): Fetches data from external HTTP/HTTPS endpoints. Requires `config.uri` field with the full URL.

### Entity

Read data from an entity definition:

```json
{
  "type": "entity",
  "entity_definition": "customer",
  "filter": {
    "field": "status",
    "operator": "=",
    "value": "active"
  },
  "mapping": {
    "entity_field": "normalized_field"
  }
}
```

### PreviousStep

Read data from the previous step's output:

```json
{
  "type": "previous_step",
  "mapping": {
    "previous_field": "normalized_field"
  }
}
```

**Note**: Cannot be used in step 0 (the first step).

### Trigger

Trigger-based input - accepts GET requests at `/api/v1/workflows/{uuid}/trigger` endpoint. No data payload - just triggers the workflow to run. Useful for cron-like external triggers or webhooks that only need to trigger execution.

```json
{
  "type": "trigger",
  "mapping": {}
}
```

**Note**: Can only be used in step 0 (the first step). When using trigger, step 2 can use static `from.uri` endpoints to pull from external APIs (no need for PreviousStep since step 1 has no data).

## Transform Types

### None

No transformation (pass-through):

```json
{
  "type": "none"
}
```

### Arithmetic

Perform arithmetic operations on numeric fields:

```json
{
  "type": "arithmetic",
  "target": "total",
  "left": { "kind": "field", "field": "price" },
  "op": "mul",
  "right": { "kind": "const", "value": 1.19 }
}
```

**Operations**: `add`, `sub`, `mul`, `div`

**Operands**:
- `{ "kind": "field", "field": "field_name" }` - Reference a normalized field
- `{ "kind": "const", "value": 123.45 }` - Use a constant number
- `{ "kind": "external_entity_field", ... }` - Read a numeric field from another
  entity, resolved at run time

**Type Casting**: String values are automatically cast to numbers when possible (e.g., `"123.45"` → `123.45`). Invalid conversions fail with clear error messages.

### Concat

Concatenate string values:

```json
{
  "type": "concat",
  "target": "full_name",
  "left": { "kind": "field", "field": "first_name" },
  "separator": " ",
  "right": { "kind": "field", "field": "last_name" }
}
```

**String Operands**:
- `{ "kind": "field", "field": "field_name" }` - Reference a normalized field
- `{ "kind": "const_string", "value": "text" }` - Use a constant string

**Type Casting**: Numeric values are automatically cast to strings (e.g., `123.0` → `"123"`, `123.45` → `"123.45"`).

### ResolveEntityPath

Look an entity up by field filters and write its path — and optionally its UUID
— into the normalized data. Runs in the services layer, so it reaches the
database.

```json
{
  "type": "resolve_entity_path",
  "target_path": "customer_path",
  "target_uuid": "customer_uuid",
  "entity_type": "customer",
  "filters": { "email": { "kind": "field", "field": "email" } },
  "value_transforms": { "email": "lowercase" },
  "fallback_path": "/customers/unknown"
}
```

| Field | Required | Notes |
|---|---|---|
| `target_path` | yes | Normalized field to receive the resolved path |
| `target_uuid` | no | Receives the entity's UUID — use as `parent_uuid` for children |
| `entity_type` | yes | Entity type to query |
| `filters` | yes | Field name → `StringOperand` |
| `value_transforms` | no | Field → transform, e.g. `lowercase`, `trim` |
| `fallback_path` | no | Used when nothing matches |

### BuildPath

Build a path from a template with `{field}` placeholders.

```json
{
  "type": "build_path",
  "target": "entity_path",
  "template": "/customers/{country}/{city}",
  "separator": "/",
  "field_transforms": { "city": "lowercase" }
}
```

`separator` defaults to `/`. `field_transforms` applies to placeholder values
before substitution.

### GetOrCreateEntity

Resolve an entity by path, creating it when absent. This one writes.

```json
{
  "type": "get_or_create_entity",
  "target_path": "category_path",
  "target_uuid": "category_uuid",
  "entity_type": "category",
  "path_template": "/categories/{category_name}",
  "create_field_data": { "name": { "kind": "field", "field": "category_name" } },
  "path_separator": "/"
}
```

### Authenticate

Verify a submitted identifier and password against entity data and issue an
entity JWT.

```json
{
  "type": "authenticate",
  "entity_type": "user",
  "identifier_field": "email",
  "password_field": "password",
  "input_identifier": "email",
  "input_password": "password",
  "target_token": "token",
  "extra_claims": { "role": "role" },
  "token_expiry_seconds": 3600
}
```

`token_expiry_seconds` falls back to the `JWT_EXPIRATION` environment variable.

### SendEmail

Send an email mid-run via a workflow email template. Requires SMTP to be
configured; without it the transform sets `target_status` and continues rather
than failing the run.

```json
{
  "type": "send_email",
  "template_uuid": "0195f0a0-...",
  "to": [{ "kind": "field", "field": "email" }],
  "cc": [{ "kind": "const_string", "value": "ops@example.com" }],
  "target_status": "email_status"
}
```

## ToDef Types

### Format

Output data as CSV, JSON, or other formats:

```json
{
  "type": "format",
  "output": { "mode": "api" },
  "format": {
    "format_type": "json",
    "options": {}
  },
  "mapping": {
    "normalized_field": "destination_field"
  }
}
```

**Output Modes**:
- `api`: Provide data via API endpoint
- `download`: Download as file
- `push`: Push to external destination (URI, etc.)

### Entity

Save data to an entity:

```json
{
  "type": "entity",
  "entity_definition": "order",
  "path": "/orders",
  "mode": "create",
  "mapping": {
    "normalized_field": "entity_field"
  }
}
```

**Modes**: `create`, `update`, `create_or_update`

For `update` and `create_or_update`, identify the existing record with either:

| Field | Notes |
|---|---|
| `identify` | An `EntityFilter`, the same shape `from.entity` uses |
| `update_key` | A field name whose value locates the record |

### Email

Send the step's produced output as an email.

```json
{
  "type": "email",
  "template_uuid": "0195f0a0-...",
  "to": [{ "kind": "field", "field": "email" }],
  "cc": [{ "kind": "const_string", "value": "ops@example.com" }],
  "mapping": { "total": "order_total" }
}
```

`mapping` maps produced fields to template variables.

### NextStep

Explicitly pass data to the next step:

```json
{
  "type": "next_step",
  "mapping": {
    "normalized_field": "next_step_field"
  }
}
```

**Note**: Cannot be used in the last step.

**Empty Mapping**: If `mapping` is empty `{}`, all normalized fields are passed through to the next step.

## Mapping

Mappings define how fields are transformed between different representations:

- **From Mapping**: `{ "source_field": "normalized_field" }` - Maps source fields to normalized names
- **To Mapping**: `{ "normalized_field": "destination_field" }` - Maps normalized fields to destination names
- **Empty Mapping**: `{}` - Passes through all fields (for both `from` and `to`)

## Examples

### Example 1: Simple Price Calculation

Calculate total price with tax:

```json
{
  "steps": [
    {
      "from": {
        "type": "format",
        "source": { "source_type": "api", "config": {} },
        "format": { "format_type": "json", "options": {} },
        "mapping": { "price": "price" }
      },
      "transform": {
        "type": "arithmetic",
        "target": "total",
        "left": { "kind": "field", "field": "price" },
        "op": "mul",
        "right": { "kind": "const", "value": 1.19 }
      },
      "to": {
        "type": "format",
        "output": { "mode": "api" },
        "format": { "format_type": "json", "options": {} },
        "mapping": {}
      }
    }
  ]
}
```

### Example 2: Chained Steps

Calculate tax and total in separate steps:

```json
{
  "steps": [
    {
      "from": {
        "type": "format",
        "source": { "source_type": "api", "config": {} },
        "format": { "format_type": "json", "options": {} },
        "mapping": { "price": "price" }
      },
      "transform": { "type": "none" },
      "to": {
        "type": "next_step",
        "mapping": { "price": "base_price" }
      }
    },
    {
      "from": {
        "type": "previous_step",
        "mapping": { "base_price": "base_price" }
      },
      "transform": {
        "type": "arithmetic",
        "target": "tax",
        "left": { "kind": "field", "field": "base_price" },
        "op": "mul",
        "right": { "kind": "const", "value": 0.19 }
      },
      "to": {
        "type": "next_step",
        "mapping": {}
      }
    },
    {
      "from": {
        "type": "previous_step",
        "mapping": { "base_price": "price", "tax": "tax" }
      },
      "transform": {
        "type": "arithmetic",
        "target": "total",
        "left": { "kind": "field", "field": "price" },
        "op": "add",
        "right": { "kind": "field", "field": "tax" }
      },
      "to": {
        "type": "format",
        "output": { "mode": "api" },
        "format": { "format_type": "json", "options": {} },
        "mapping": {}
      }
    }
  ]
}
```

### Example 3: Fan-Out Pattern

One input produces multiple calculations:

```json
{
  "steps": [
    {
      "from": {
        "type": "format",
        "source": { "source_type": "api", "config": {} },
        "format": { "format_type": "json", "options": {} },
        "mapping": { "price": "price" }
      },
      "transform": { "type": "none" },
      "to": {
        "type": "format",
        "output": { "mode": "api" },
        "format": { "format_type": "json", "options": {} },
        "mapping": {}
      }
    },
    {
      "from": {
        "type": "previous_step",
        "mapping": { "price": "price" }
      },
      "transform": {
        "type": "arithmetic",
        "target": "discounted_price",
        "left": { "kind": "field", "field": "price" },
        "op": "mul",
        "right": { "kind": "const", "value": 0.9 }
      },
      "to": {
        "type": "format",
        "output": { "mode": "api" },
        "format": { "format_type": "json", "options": {} },
        "mapping": {}
      }
    },
    {
      "from": {
        "type": "previous_step",
        "mapping": { "price": "price" }
      },
      "transform": {
        "type": "arithmetic",
        "target": "taxed_price",
        "left": { "kind": "field", "field": "price" },
        "op": "mul",
        "right": { "kind": "const", "value": 1.1 }
      },
      "to": {
        "type": "format",
        "output": { "mode": "api" },
        "format": { "format_type": "json", "options": {} },
        "mapping": {}
      }
    }
  ]
}
```

### Example 4: String Concatenation

Combine first and last name:

```json
{
  "steps": [
    {
      "from": {
        "type": "format",
        "source": { "source_type": "api", "config": {} },
        "format": { "format_type": "json", "options": {} },
        "mapping": {
          "first_name": "first_name",
          "last_name": "last_name"
        }
      },
      "transform": {
        "type": "concat",
        "target": "full_name",
        "left": { "kind": "field", "field": "first_name" },
        "separator": " ",
        "right": { "kind": "field", "field": "last_name" }
      },
      "to": {
        "type": "format",
        "output": { "mode": "api" },
        "format": { "format_type": "json", "options": {} },
        "mapping": {}
      }
    }
  ]
}
```

### Example 5: Trigger → External API → Entity Creation

Trigger workflow via GET request at `/api/v1/workflows/{uuid}/trigger`, fetch data from external API, and create entities:

```json
{
  "steps": [
    {
      "from": {
        "type": "trigger",
        "mapping": {}
      },
      "transform": {
        "type": "none"
      },
      "to": {
        "type": "next_step",
        "mapping": {}
      }
    },
    {
      "from": {
        "type": "format",
        "source": {
          "source_type": "uri",
          "config": {
            "uri": "https://api.example.com/data"
          },
          "auth": null
        },
        "format": {
          "format_type": "json",
          "options": {}
        },
        "mapping": {
          "name": "name",
          "email": "email"
        }
      },
      "transform": {
        "type": "none"
      },
      "to": {
        "type": "entity",
        "entity_definition": "customer",
        "path": "/",
        "mode": "create",
        "mapping": {
          "name": "name",
          "email": "email"
        }
      }
    }
  ]
}
```

**Note**: When using `trigger` type, step 2 doesn't need `PreviousStep` since step 1 has no data to pass. Step 2 can use static `from.uri` endpoints to pull from external APIs. The trigger endpoint is `GET /api/v1/workflows/{uuid}/trigger`, while Provider workflows use `GET /api/v1/workflows/{uuid}` to fetch data.

## Post-Run Hooks (`on_complete`)

A program may carry an `on_complete` section alongside `steps`, describing
actions to take once the whole run finishes.

```json
{
  "steps": [ ... ],
  "on_complete": {
    "actions": [
      {
        "type": "send_email",
        "template_uuid": "0195f0a0-...",
        "to": [{ "kind": "const_string", "value": "ops@example.com" }],
        "cc": null,
        "condition": "on_failure"
      }
    ]
  }
}
```

**Conditions**: `always` (default), `on_success` (no failed items),
`on_failure` (at least one failed item).

Recipients here must be `const_string`. There is no per-item context after the
run, so field references have nothing to resolve against.

## Type Casting Rules

### String to Number (for Arithmetic)

- Valid: `"123"` → `123.0`, `"123.45"` → `123.45`, `"-10.5"` → `-10.5`
- Invalid: `"abc"`, `"123abc"`, `""` (empty string) → Error with clear message

### Number to String (for Concatenation)

- Integer-valued floats: `123.0` → `"123"` (not `"123.0"`)
- Decimal floats: `123.45` → `"123.45"`
- Zero: `0` → `"0"`
- Negative: `-15.5` → `"-15.5"`

## Validation Rules

1. **Steps Array**: Must contain at least one step
2. **PreviousStep**: Cannot be used in step 0
3. **NextStep**: Cannot be used in the last step
4. **Field Names**: Must match pattern `^[A-Za-z_][A-Za-z0-9_\.]*$`
5. **Arithmetic**: Operands must be numeric (strings are cast, but invalid casts fail)
6. **Division**: Division by zero is not allowed

## Error Handling

- **Type Casting Errors**: Clear messages like "cannot convert string 'abc' to number"
- **Missing Fields**: Errors indicate which field is missing
- **Null Values**: Null fields in arithmetic/concat operations produce errors
- **Division by Zero**: Explicit error message

## Best Practices

1. **Use NextStep Explicitly**: When chaining steps, use `NextStep` ToDef to make data flow explicit
2. **Empty Mappings**: Use empty mappings `{}` to pass through all fields when appropriate
3. **Field Naming**: Use descriptive normalized field names for clarity
4. **Step Organization**: Group related transformations in sequential steps
5. **Error Prevention**: Validate field names and types before execution

## See Also

- [Example Files](../.example_files/json_examples/dsl/) - More complete examples
- [Test Fixtures](../.example_files/json_examples/dsl/tests/) - Test cases demonstrating various patterns

