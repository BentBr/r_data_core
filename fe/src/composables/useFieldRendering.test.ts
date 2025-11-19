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
                constraints: { min: 5, max: 10 },
            }

            const rules = getFieldRules(field)

            expect(rules.length).toBe(3) // required, min, max
        })
    })

    describe('getFieldIcon', () => {
        it('should return correct icon for String field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('String')).toBe('mdi-text')
        })

        it('should return correct icon for Boolean field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('Boolean')).toBe('mdi-checkbox-marked')
        })

        it('should return correct icon for Date field', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('Date')).toBe('mdi-calendar')
        })

        it('should return default icon for unknown field type', () => {
            const { getFieldIcon } = useFieldRendering()
            expect(getFieldIcon('UnknownType')).toBe('mdi-text')
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
})
