import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useFieldRendering } from './useFieldRendering'
import type { FieldDefinition } from '@/types/schemas'

// Mock useTranslations
const mockT = vi.fn((key: string) => key)

vi.mock('./useTranslations', () => ({
    useTranslations: () => ({
        t: mockT,
    }),
}))

describe('useFieldRendering', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    describe('getFieldComponent', () => {
        it('should return correct component for String field', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('String')).toBe('v-text-field')
        })

        it('should return correct component for Text field', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('Text')).toBe('v-textarea')
        })

        it('should return correct component for Integer field', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('Integer')).toBe('v-text-field')
        })

        it('should return correct component for Boolean field', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('Boolean')).toBe('v-checkbox')
        })

        it('should return correct component for Select field', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('Select')).toBe('v-select')
        })

        it('should return correct component for Json field', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('Json')).toBe('v-textarea')
        })

        it('should return correct component for Object field', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('Object')).toBe('v-textarea')
        })

        it('should return correct component for Array field', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('Array')).toBe('v-textarea')
        })

        it('should return default component for unknown field type', () => {
            const { getFieldComponent } = useFieldRendering()
            expect(getFieldComponent('UnknownType')).toBe('v-text-field')
        })
    })

    describe('getFieldRules', () => {
        it('should add required rule when field is required', () => {
            const { getFieldRules } = useFieldRendering()
            const field: FieldDefinition = {
                name: 'test',
                display_name: 'Test Field',
                field_type: 'String',
                required: true,
                indexed: false,
                filterable: false,
            }

            const rules = getFieldRules(field)

            expect(rules.length).toBe(1)
            expect(rules[0]('')).toBe('Test Field is required')
            expect(rules[0]('value')).toBe(true)
        })

        it('should add min constraint rule', () => {
            const { getFieldRules } = useFieldRendering()
            const field: FieldDefinition = {
                name: 'test',
                display_name: 'Test Field',
                field_type: 'Integer',
                required: false,
                indexed: false,
                filterable: false,
                constraints: { min: 5 },
            }

            const rules = getFieldRules(field)

            expect(rules.length).toBe(1)
            expect(rules[0](3)).toBe('Minimum value is 5')
            expect(rules[0](5)).toBe(true)
            expect(rules[0](10)).toBe(true)
            expect(rules[0](null)).toBe(true) // Empty values pass
        })

        it('should add max constraint rule', () => {
            const { getFieldRules } = useFieldRendering()
            const field: FieldDefinition = {
                name: 'test',
                display_name: 'Test Field',
                field_type: 'Integer',
                required: false,
                indexed: false,
                filterable: false,
                constraints: { max: 10 },
            }

            const rules = getFieldRules(field)

            expect(rules.length).toBe(1)
            expect(rules[0](15)).toBe('Maximum value is 10')
            expect(rules[0](10)).toBe(true)
            expect(rules[0](5)).toBe(true)
        })

        it('should add pattern constraint rule', () => {
            const { getFieldRules } = useFieldRendering()
            const field: FieldDefinition = {
                name: 'test',
                display_name: 'Test Field',
                field_type: 'String',
                required: false,
                indexed: false,
                filterable: false,
                constraints: { pattern: '^[A-Z]+$' },
            }

            const rules = getFieldRules(field)

            expect(rules.length).toBe(1)
            expect(rules[0]('ABC')).toBe(true)
            expect(rules[0]('abc')).toBe('Invalid format for Test Field')
            expect(rules[0]('')).toBe(true) // Empty values pass
        })

        it('should combine multiple rules', () => {
            const { getFieldRules } = useFieldRendering()
            const field: FieldDefinition = {
                name: 'test',
                display_name: 'Test Field',
                field_type: 'Integer',
                required: true,
                indexed: false,
                filterable: false,
                constraints: { min: 5, max: 10 },
            }

            const rules = getFieldRules(field)

            expect(rules.length).toBe(3) // required, min, max
        })
    })

    describe('getFieldIcon', () => {
        it('should return correct icon for String field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('String')).toBe('type')
        })

        it('should return correct icon for Boolean field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('Boolean')).toBe('check-square')
        })

        it('should return correct icon for Date field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('Date')).toBe('calendar')
        })

        it('should return correct icon for Json field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('Json')).toBe('braces')
        })

        it('should return correct icon for Object field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('Object')).toBe('box')
        })

        it('should return correct icon for Array field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('Array')).toBe('list')
        })

        it('should return default icon for unknown field type', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('UnknownType')).toBe('type')
        })
    })

    describe('formatFieldValue', () => {
        it('should return empty string for null/undefined', () => {
            const { formatFieldValue } = useFieldRendering()
            expect(formatFieldValue(null, 'String')).toBe('common.empty')
            expect(formatFieldValue(undefined, 'String')).toBe('common.empty')
        })

        it('should format Boolean values', () => {
            const { formatFieldValue } = useFieldRendering()
            expect(formatFieldValue(true, 'Boolean')).toBe('common.yes')
            expect(formatFieldValue(false, 'Boolean')).toBe('common.no')
        })

        it('should format Date values', () => {
            const { formatFieldValue } = useFieldRendering()
            const date = '2024-01-15'
            const formatted = formatFieldValue(date, 'Date')
            expect(formatted).toMatch(/1\/15\/2024|15\/1\/2024/) // Locale-dependent
        })

        it('should format DateTime values', () => {
            const { formatFieldValue } = useFieldRendering()
            const dateTime = '2024-01-15T10:30:00'
            const formatted = formatFieldValue(dateTime, 'DateTime')
            expect(formatted).toMatch(/1\/15\/2024|15\/1\/2024/)
        })

        it('should format Time values', () => {
            const { formatFieldValue } = useFieldRendering()
            const time = '10:30:00'
            const formatted = formatFieldValue(time, 'Time')
            expect(typeof formatted).toBe('string')
            expect(formatted.length).toBeGreaterThan(0)
        })

        it('should format Json/Object values', () => {
            const { formatFieldValue } = useFieldRendering()
            const obj = { key: 'value' }
            expect(formatFieldValue(obj, 'Json')).toBe('{"key":"value"}')
            expect(formatFieldValue(obj, 'Object')).toBe('{"key":"value"}')
        })

        it('should format Json values with complex nested structures', () => {
            const { formatFieldValue } = useFieldRendering()
            const complexObj = {
                count: 10,
                names: ['Customer', 'Order'],
                metadata: { version: 1 },
            }
            const formatted = formatFieldValue(complexObj, 'Json')
            expect(formatted).toContain('"count":10')
            expect(formatted).toContain('"names"')
        })

        it('should format Json array values', () => {
            const { formatFieldValue } = useFieldRendering()
            const arr = [{ entity_type: 'Customer', count: 100 }]
            const formatted = formatFieldValue(arr, 'Json')
            expect(formatted).toContain('"entity_type":"Customer"')
            expect(formatted).toContain('"count":100')
        })

        it('should format Array values', () => {
            const { formatFieldValue } = useFieldRendering()
            const arr = [1, 2, 3]
            expect(formatFieldValue(arr, 'Array')).toBe('[3 items]')
        })

        it('should format default values as string', () => {
            const { formatFieldValue } = useFieldRendering()
            expect(formatFieldValue(123, 'String')).toBe('123')
            expect(formatFieldValue('text', 'String')).toBe('text')
        })

        // Edge case tests: object/array values with non-JSON field types
        // This handles misconfigured field types where value is object but type is String

        it('should handle object values with String field type (edge case)', () => {
            const { formatFieldValue } = useFieldRendering()
            const value = [
                { entity_type: 'Puh', count: 7 },
                { entity_type: 'customer', count: 3006 },
            ]
            const result = formatFieldValue(value, 'String')
            // Should NOT produce "[object Object],[object Object]"
            expect(result).not.toContain('[object Object]')
            expect(result).toBe(JSON.stringify(value))
        })

        it('should handle array values with Text field type (edge case)', () => {
            const { formatFieldValue } = useFieldRendering()
            const value = ['*', 'https://example.com']
            const result = formatFieldValue(value, 'Text')
            expect(result).not.toContain('[object Object]')
            expect(result).toBe(JSON.stringify(value))
        })

        it('should handle nested object values with unknown field type', () => {
            const { formatFieldValue } = useFieldRendering()
            const value = { count: 10, names: ['Test'] }
            const result = formatFieldValue(value, 'UnknownType')
            expect(result).toBe(JSON.stringify(value))
        })

        it('should handle entity count array with incorrect String field type', () => {
            // This is the exact edge case from the statistics submission
            const { formatFieldValue } = useFieldRendering()
            const value = [
                { entity_type: 'Puh', count: 7 },
                { entity_type: 'customer', count: 3006 },
                { entity_type: 'user', count: 1 },
            ]
            const result = formatFieldValue(value, 'String')
            expect(result).not.toContain('[object Object]')
            expect(result).toContain('"entity_type":"Puh"')
            expect(result).toContain('"count":3006')
        })
    })

    describe('getInputType', () => {
        it('should return correct input type for String', () => {
            const { getInputType } = useFieldRendering()
            expect(getInputType('String')).toBe('text')
        })

        it('should return correct input type for Integer', () => {
            const { getInputType } = useFieldRendering()
            expect(getInputType('Integer')).toBe('number')
        })

        it('should return correct input type for Email', () => {
            const { getInputType } = useFieldRendering()
            expect(getInputType('Email')).toBe('email')
        })

        it('should return correct input type for Date', () => {
            const { getInputType } = useFieldRendering()
            expect(getInputType('Date')).toBe('date')
        })

        it('should return default type for unknown field type', () => {
            const { getInputType } = useFieldRendering()
            expect(getInputType('UnknownType')).toBe('text')
        })
    })

    describe('isJsonFieldType', () => {
        it('should return true for Json field type', () => {
            const { isJsonFieldType } = useFieldRendering()
            expect(isJsonFieldType('Json')).toBe(true)
        })

        it('should return true for Object field type', () => {
            const { isJsonFieldType } = useFieldRendering()
            expect(isJsonFieldType('Object')).toBe(true)
        })

        it('should return true for Array field type', () => {
            const { isJsonFieldType } = useFieldRendering()
            expect(isJsonFieldType('Array')).toBe(true)
        })

        it('should return false for String field type', () => {
            const { isJsonFieldType } = useFieldRendering()
            expect(isJsonFieldType('String')).toBe(false)
        })

        it('should return false for Integer field type', () => {
            const { isJsonFieldType } = useFieldRendering()
            expect(isJsonFieldType('Integer')).toBe(false)
        })

        it('should return false for unknown field type', () => {
            const { isJsonFieldType } = useFieldRendering()
            expect(isJsonFieldType('Unknown')).toBe(false)
        })
    })

    describe('parseJsonFieldValue', () => {
        it('should return value unchanged for non-JSON field types', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('hello', 'String')
            expect(result.parsed).toBe('hello')
            expect(result.error).toBeNull()
        })

        it('should return object unchanged if already an object', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const obj = { key: 'value' }
            const result = parseJsonFieldValue(obj, 'Json')
            expect(result.parsed).toEqual(obj)
            expect(result.error).toBeNull()
        })

        it('should return array unchanged if already an array', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const arr = [1, 2, 3]
            const result = parseJsonFieldValue(arr, 'Array')
            expect(result.parsed).toEqual(arr)
            expect(result.error).toBeNull()
        })

        it('should return null/undefined/empty string unchanged', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            expect(parseJsonFieldValue(null, 'Json').parsed).toBeNull()
            expect(parseJsonFieldValue(undefined, 'Json').parsed).toBeUndefined()
            expect(parseJsonFieldValue('', 'Json').parsed).toBe('')
        })

        it('should parse valid JSON object string for Json type', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('{"order_items": {"0": "product1"}}', 'Json')
            expect(result.parsed).toEqual({ order_items: { '0': 'product1' } })
            expect(result.error).toBeNull()
        })

        it('should parse valid JSON object string for Object type', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('{"name": "test", "count": 5}', 'Object')
            expect(result.parsed).toEqual({ name: 'test', count: 5 })
            expect(result.error).toBeNull()
        })

        it('should parse valid JSON array string for Array type', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('["a", "b", "c"]', 'Array')
            expect(result.parsed).toEqual(['a', 'b', 'c'])
            expect(result.error).toBeNull()
        })

        it('should return error for invalid JSON string', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('not valid json', 'Json')
            expect(result.parsed).toBe('not valid json')
            expect(result.error).toBe('Must be valid JSON')
        })

        it('should return error when Array type receives non-array JSON', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('{"key": "value"}', 'Array')
            expect(result.parsed).toBe('{"key": "value"}')
            expect(result.error).toBe('Must be a valid JSON array')
        })

        it('should return error when Object type receives array JSON', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('[1, 2, 3]', 'Object')
            expect(result.parsed).toBe('[1, 2, 3]')
            expect(result.error).toBe('Must be a valid JSON object')
        })

        it('should return error when Json type receives non-object JSON', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('"just a string"', 'Json')
            expect(result.parsed).toBe('"just a string"')
            expect(result.error).toBe('Must be a valid JSON object')
        })

        it('should handle nested JSON structures', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const nestedJson = '{"items": [{"id": 1}, {"id": 2}], "meta": {"total": 2}}'
            const result = parseJsonFieldValue(nestedJson, 'Json')
            expect(result.parsed).toEqual({
                items: [{ id: 1 }, { id: 2 }],
                meta: { total: 2 },
            })
            expect(result.error).toBeNull()
        })

        it('should handle JSON with special characters', () => {
            const { parseJsonFieldValue } = useFieldRendering()
            const result = parseJsonFieldValue('{"message": "Hello \\"world\\""}', 'Json')
            expect(result.parsed).toEqual({ message: 'Hello "world"' })
            expect(result.error).toBeNull()
        })
    })

    describe('stringifyJsonFieldValue', () => {
        it('should return value as string for non-JSON field types', () => {
            const { stringifyJsonFieldValue } = useFieldRendering()
            expect(stringifyJsonFieldValue('hello', 'String')).toBe('hello')
            expect(stringifyJsonFieldValue(123, 'Integer')).toBe('123')
        })

        it('should return empty string for null/undefined', () => {
            const { stringifyJsonFieldValue } = useFieldRendering()
            expect(stringifyJsonFieldValue(null, 'Json')).toBe('')
            expect(stringifyJsonFieldValue(undefined, 'Object')).toBe('')
        })

        it('should stringify object with pretty formatting', () => {
            const { stringifyJsonFieldValue } = useFieldRendering()
            const obj = { key: 'value' }
            const result = stringifyJsonFieldValue(obj, 'Json')
            expect(result).toBe('{\n  "key": "value"\n}')
        })

        it('should stringify array with pretty formatting', () => {
            const { stringifyJsonFieldValue } = useFieldRendering()
            const arr = [1, 2, 3]
            const result = stringifyJsonFieldValue(arr, 'Array')
            expect(result).toBe('[\n  1,\n  2,\n  3\n]')
        })

        it('should stringify nested object with pretty formatting', () => {
            const { stringifyJsonFieldValue } = useFieldRendering()
            const obj = { items: [{ id: 1 }], meta: { count: 1 } }
            const result = stringifyJsonFieldValue(obj, 'Object')
            expect(result).toContain('"items"')
            expect(result).toContain('"meta"')
            expect(result.split('\n').length).toBeGreaterThan(1)
        })

        it('should return string value unchanged for JSON field type', () => {
            const { stringifyJsonFieldValue } = useFieldRendering()
            // If value is already a string (e.g., user input), return as-is
            expect(stringifyJsonFieldValue('{"key": "value"}', 'Json')).toBe('{"key": "value"}')
        })

        it('should handle empty object', () => {
            const { stringifyJsonFieldValue } = useFieldRendering()
            expect(stringifyJsonFieldValue({}, 'Json')).toBe('{}')
        })

        it('should handle empty array', () => {
            const { stringifyJsonFieldValue } = useFieldRendering()
            expect(stringifyJsonFieldValue([], 'Array')).toBe('[]')
        })
    })
})
