# DSL Calculations and Step Chaining - Examples and Test Data

This directory contains example files demonstrating the DSL (Domain Specific Language) workflow capabilities, specifically focusing on:

1. **Step Chaining** - Steps can read from previous step outputs
2. **Type Casting** - Automatic string↔number conversion
3. **Field Dropdowns** - UI provides field selection dropdowns
4. **Fan-out Patterns** - One input can produce multiple calculated outputs

## CSV Example Files

### `/csv_examples/products_with_prices.csv`
Product inventory data with numeric prices and quantities.
- **Use case**: Calculate total inventory value, apply discounts
- **Fields**: `product_id`, `product_name`, `price`, `quantity`

### `/csv_examples/sales_data_string_numbers.csv`
Sales data where numeric fields are stored as strings (common in CSV imports).
- **Use case**: Demonstrates automatic string-to-number casting for calculations
- **Fields**: `sale_id`, `product`, `units_sold` (string), `unit_price` (string)

### `/csv_examples/customer_data.csv`
Customer information including demographic data.
- **Use case**: String concatenation (full names), age calculations
- **Fields**: `customer_id`, `first_name`, `last_name`, `age`, `country`

## JSON Workflow Examples

### 1. **workflow_chained_price_calculation.json**
**Feature**: Multi-step calculation pipeline
- **Step 0**: Read product data (price, quantity)
- **Step 1**: Calculate total_value = price × quantity (reads from Step 0)
- **Step 2**: Calculate total_with_tax = total_value × 1.19 (reads from Step 1)
- **Output**: Saves to entity with accumulated calculations

**Key Concept**: Each step builds on the previous step's output, creating a calculation pipeline.

### 2. **workflow_fanout_customer_processing.json**
**Feature**: Fan-out pattern with mixed operations
- **Step 0**: Read customer data
- **Step 1**: Concatenate first_name + last_name → full_name
- **Step 2**: Calculate age from birth_year (2025 - birth_year)
- **Output**: Entity with both calculated fields

**Key Concept**: One input produces multiple independent calculations, all saved together.

### 3. **workflow_string_to_number_casting.json**
**Feature**: Automatic type casting for CSV string numbers
- **Step 0**: Read CSV with string numbers ("5", "12.50")
- **Transform**: Arithmetic multiplication automatically casts strings to numbers
- **Output**: Calculated numeric total

**Key Concept**: DSL automatically converts string fields to numbers for arithmetic operations.

### 4. **workflow_number_to_string_concat.json**
**Feature**: Automatic number-to-string casting for concatenation
- **Step 0**: Read order data with numeric total
- **Transform**: Concatenate "Order #" + numeric_field → string
- **Output**: Formatted string like "Order #123 - Total: $45.99"

**Key Concept**: Numbers are automatically formatted as strings for concatenation. Special formatting: `100.0` → `"100"`, `100.50` → `"100.5"`

### 5. **workflow_multi_step_discount_calculation.json**
**Feature**: Complex multi-step calculation with field accumulation
- **Step 0**: Read original_price, discount_percent
- **Step 1**: Calculate discount_amount = original_price × discount_percent
- **Step 2**: Calculate final_price = original_price - discount_amount
- **Output**: Entity with all fields (original, discount, final)

**Key Concept**: Each step can access fields from the original input AND calculated fields from previous steps.

### 6. **workflow_previous_step_accumulated_fields.json**
**Feature**: Progressive field accumulation across 4 steps
- **Step 0**: Read base_salary
- **Step 1**: Calculate health_insurance (5% of base)
- **Step 2**: Calculate retirement_contrib (10% of base)
- **Step 3**: Calculate total_benefits = health + retirement
- **Output**: Entity with all accumulated fields

**Key Concept**: Demonstrates how fields accumulate through multiple steps, all available in the final output.

### 7. **workflow_unicode_concat.json**
**Feature**: Unicode and emoji support in string concatenation
- **Transform**: Concatenate emoji + message (handles UTF-8)
- **Example**: "🚀" + "こんにちは" → "🚀 こんにちは"

**Key Concept**: DSL fully supports Unicode characters in all string operations.

### 8. **workflow_empty_mapping_passthrough.json**
**Feature**: Empty mappings pass through all fields
- **Step 0**: Calculate result, empty to.mapping → passes all fields
- **Step 1**: Empty from.mapping → receives all fields from Step 0

**Key Concept**: Empty mappings `{}` act as "pass through all fields", useful for intermediate steps.

## Testing and Validation

All examples are validated by comprehensive integration tests:

### Test Coverage (56 tests total)
- **Step Chaining**: 6 tests (simple, complex, validation)
- **Fan-out Patterns**: 5 tests (parallel calculations, field propagation)
- **Type Casting**: 16 tests
  - String → Number: integers, floats, scientific notation, whitespace, negatives
  - Number → String: integers, floats, zero, negatives, large numbers
  - Invalid conversions: empty strings, alphabetic, alphanumeric
- **Edge Cases**: 10 tests
  - Null fields, missing fields, empty input
  - Very large/small numbers, long strings, Unicode
  - Special characters, boolean fields

### Quality Gates ✅
- `cargo clippy`: No warnings
- `cargo test`: All 56 DSL integration tests pass
- `npm run lint`: TypeScript compilation clean
- `npm run test:run`: All 433 frontend tests pass

## Type Casting Rules

### Arithmetic Operations (String → Number)
- **Valid**: `"42"` → `42.0`, `"123.45"` → `123.45`, `"-10.5"` → `-10.5`
- **Valid**: `"  42  "` → `42.0` (whitespace trimmed)
- **Valid**: `"1.5e2"` → `150.0` (scientific notation)
- **Invalid**: `""`, `"abc"`, `"123abc"`, `true` → Error with clear message

### Concatenation (Number → String)
- **Smart Formatting**: `100.0` → `"100"` (no decimal point for whole numbers)
- **Preserve Decimals**: `100.50` → `"100.5"`, `19.99` → `"19.99"`
- **Negatives**: `-15.5` → `"-15.5"`
- **Large Numbers**: `7800000000` → `"7800000000"`
- **Zero**: `0` → `"0"`

## UI Features

### Field Dropdowns
When creating calculations in the UI:
1. **From Definition**: Dropdown shows available source fields
2. **Transform**: Dropdowns for operands show normalized fields
3. **To Definition**: Dropdown shows calculated + normalized fields

### Step Context
- Step 0: Can only use `Format` or `Entity` as input
- Step N (N > 0): Can use `Format`, `Entity`, OR `PreviousStep`
- PreviousStep shows available fields from previous step's output

### Validation
- UI warns if trying to use PreviousStep in Step 0
- Info banner explains step chaining when selected
- Real-time field availability based on step configuration

## Architecture Notes

### Step Execution Flow
```
Step 0: Input → Normalize → Transform → Output
                                         ↓
Step 1: ←――――――――――――――――――――――――――――――┘ → Normalize → Transform → Output
                                                                     ↓
Step 2: ←―――――――――――――――――――――――――――――――――――――――――――――――――――――――┘
```

### Normalized Data
Each step maintains a "normalized" JSON object containing:
- Mapped fields from `FromDef`
- Calculated fields from `Transform`
- This normalized data is passed to the next step

### Empty Mappings
- Empty `from.mapping`: Pass through ALL source fields
- Empty `to.mapping`: Pass through ALL normalized fields
- Useful for intermediate steps that just add calculations

## Common Patterns

### Pattern 1: Simple Pipeline
```
Input → Calculate A → Calculate B (uses A) → Save
```

### Pattern 2: Fan-out
```
Input → Calculate A ──→ Save A
      ↓
      └──→ Calculate B → Save B
```

### Pattern 3: Accumulation
```
Input → +Field1 → +Field2 → +Field3 → Save All
```

### Pattern 4: Transform & Format
```
CSV (strings) → Calculate → Format → Output JSON
```

## Error Handling

All calculation errors provide clear, actionable messages:
- `"Field 'price': cannot convert string 'abc' to number"`
- `"Field 'age' is null, expected a number"`
- `"Step 2: Division by zero in target field 'ratio'"`
- `"Step 0 cannot use PreviousStep source"`

## Performance

- Calculations execute in memory (no database round-trips)
- Large datasets: Stream processing (not shown in examples)
- Unicode strings: Full UTF-8 support with no performance penalty
- Very large numbers: Standard IEEE 754 double precision (±10^308)

