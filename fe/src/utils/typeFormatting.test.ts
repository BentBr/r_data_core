import { describe, it, expect } from 'vitest'
import { formatValueToType, formatFieldData } from './typeFormatting'
import type { FieldType } from './typeFormatting'

describe('typeFormatting', () => {
    describe('formatValueToType', () => {
        describe('Boolean type', () => {
            it('should return boolean true for boolean true', () => {
                expect(formatValueToType(true, 'Boolean')).toBe(true)
            })

            it('should return boolean false for boolean false', () => {
                expect(formatValueToType(false, 'Boolean')).toBe(false)
            })

            it('should convert string "true" to boolean true', () => {
                expect(formatValueToType('true', 'Boolean')).toBe(true)
            })

            it('should convert string "True" to boolean true', () => {
                expect(formatValueToType('True', 'Boolean')).toBe(true)
            })

            it('should convert string "TRUE" to boolean true', () => {
                expect(formatValueToType('TRUE', 'Boolean')).toBe(true)
            })

            it('should convert string "1" to boolean true', () => {
                expect(formatValueToType('1', 'Boolean')).toBe(true)
            })

            it('should convert string "yes" to boolean true', () => {
                expect(formatValueToType('yes', 'Boolean')).toBe(true)
            })

            it('should convert string "on" to boolean true', () => {
                expect(formatValueToType('on', 'Boolean')).toBe(true)
            })

            it('should convert string "false" to boolean false', () => {
                expect(formatValueToType('false', 'Boolean')).toBe(false)
            })

            it('should convert string "0" to boolean false', () => {
                expect(formatValueToType('0', 'Boolean')).toBe(false)
            })

            it('should convert string "no" to boolean false', () => {
                expect(formatValueToType('no', 'Boolean')).toBe(false)
            })

            it('should convert number 1 to boolean true', () => {
                expect(formatValueToType(1, 'Boolean')).toBe(true)
            })

            it('should convert number 0 to boolean false', () => {
                expect(formatValueToType(0, 'Boolean')).toBe(false)
            })

            it('should convert number 42 to boolean true', () => {
                expect(formatValueToType(42, 'Boolean')).toBe(true)
            })

            it('should return null for null input', () => {
                expect(formatValueToType(null, 'Boolean')).toBe(null)
            })

            it('should return null for undefined input', () => {
                expect(formatValueToType(undefined, 'Boolean')).toBe(null)
            })

            it('should return null for empty string', () => {
                expect(formatValueToType('', 'Boolean')).toBe(null)
            })

            it('should default to false for invalid string', () => {
                expect(formatValueToType('invalid', 'Boolean')).toBe(false)
            })
        })

        describe('Integer type', () => {
            it('should return integer for number input', () => {
                expect(formatValueToType(42, 'Integer')).toBe(42)
            })

            it('should floor float numbers', () => {
                expect(formatValueToType(42.7, 'Integer')).toBe(42)
            })

            it('should convert string "42" to integer 42', () => {
                expect(formatValueToType('42', 'Integer')).toBe(42)
            })

            it('should convert string "-10" to integer -10', () => {
                expect(formatValueToType('-10', 'Integer')).toBe(-10)
            })

            it('should return null for invalid string', () => {
                expect(formatValueToType('invalid', 'Integer')).toBe(null)
            })

            it('should return null for null input', () => {
                expect(formatValueToType(null, 'Integer')).toBe(null)
            })

            it('should return null for undefined input', () => {
                expect(formatValueToType(undefined, 'Integer')).toBe(null)
            })
        })

        describe('Float type', () => {
            it('should return float for number input', () => {
                expect(formatValueToType(3.14, 'Float')).toBe(3.14)
            })

            it('should convert string "3.14" to float 3.14', () => {
                expect(formatValueToType('3.14', 'Float')).toBe(3.14)
            })

            it('should convert string "-2.5" to float -2.5', () => {
                expect(formatValueToType('-2.5', 'Float')).toBe(-2.5)
            })

            it('should return null for invalid string', () => {
                expect(formatValueToType('invalid', 'Float')).toBe(null)
            })

            it('should return null for null input', () => {
                expect(formatValueToType(null, 'Float')).toBe(null)
            })

            it('should return null for undefined input', () => {
                expect(formatValueToType(undefined, 'Float')).toBe(null)
            })
        })

        describe('Date type', () => {
            it('should return string for valid date string', () => {
                expect(formatValueToType('2024-01-01', 'Date')).toBe('2024-01-01')
            })

            it('should return null for non-string input', () => {
                expect(formatValueToType(123, 'Date')).toBe(null)
            })

            it('should return null for null input', () => {
                expect(formatValueToType(null, 'Date')).toBe(null)
            })
        })

        describe('DateTime type', () => {
            it('should return string for valid datetime string', () => {
                expect(formatValueToType('2024-01-01T12:00:00', 'DateTime')).toBe(
                    '2024-01-01T12:00:00'
                )
            })

            it('should return null for non-string input', () => {
                expect(formatValueToType(123, 'DateTime')).toBe(null)
            })

            it('should return null for null input', () => {
                expect(formatValueToType(null, 'DateTime')).toBe(null)
            })
        })

        describe('Object type', () => {
            it('should return object for object input', () => {
                const obj = { key: 'value' }
                expect(formatValueToType(obj, 'Object')).toEqual(obj)
            })

            it('should parse JSON string to object', () => {
                const jsonString = '{"key":"value"}'
                expect(formatValueToType(jsonString, 'Object')).toEqual({ key: 'value' })
            })

            it('should return null for invalid JSON string', () => {
                expect(formatValueToType('invalid json', 'Object')).toBe(null)
            })

            it('should return null for null input', () => {
                expect(formatValueToType(null, 'Object')).toBe(null)
            })
        })

        describe('Array type', () => {
            it('should return array for array input', () => {
                const arr = [1, 2, 3]
                expect(formatValueToType(arr, 'Array')).toEqual(arr)
            })

            it('should parse JSON string to array', () => {
                const jsonString = '[1,2,3]'
                expect(formatValueToType(jsonString, 'Array')).toEqual([1, 2, 3])
            })

            it('should return null for invalid JSON string', () => {
                expect(formatValueToType('invalid json', 'Array')).toBe(null)
            })

            it('should return null for null input', () => {
                expect(formatValueToType(null, 'Array')).toBe(null)
            })
        })

        describe('String type', () => {
            it('should return string for string input', () => {
                expect(formatValueToType('test', 'String')).toBe('test')
            })

            it('should convert number to string', () => {
                expect(formatValueToType(123, 'String')).toBe('123')
            })

            it('should convert boolean to string', () => {
                expect(formatValueToType(true, 'String')).toBe('true')
            })
        })
    })

    describe('formatFieldData', () => {
        it('should format all fields based on field definitions', () => {
            const fieldData = {
                is_active: 'true',
                age: '25',
                price: '19.99',
                name: 'Test',
            }

            const fieldDefinitions = [
                { name: 'is_active', field_type: 'Boolean' as FieldType },
                { name: 'age', field_type: 'Integer' as FieldType },
                { name: 'price', field_type: 'Float' as FieldType },
                { name: 'name', field_type: 'String' as FieldType },
            ]

            const formatted = formatFieldData(fieldData, fieldDefinitions)

            expect(formatted.is_active).toBe(true)
            expect(formatted.age).toBe(25)
            expect(formatted.price).toBe(19.99)
            expect(formatted.name).toBe('Test')
        })

        it('should handle missing field definitions', () => {
            const fieldData = {
                is_active: 'true',
                unknown_field: 'value',
            }

            const fieldDefinitions = [{ name: 'is_active', field_type: 'Boolean' as FieldType }]

            const formatted = formatFieldData(fieldData, fieldDefinitions)

            expect(formatted.is_active).toBe(true)
            expect(formatted.unknown_field).toBe('value') // Kept as-is
        })

        it('should handle null and undefined values', () => {
            const fieldData = {
                is_active: null,
                age: undefined,
                name: '',
            }

            const fieldDefinitions = [
                { name: 'is_active', field_type: 'Boolean' as FieldType },
                { name: 'age', field_type: 'Integer' as FieldType },
                { name: 'name', field_type: 'String' as FieldType },
            ]

            const formatted = formatFieldData(fieldData, fieldDefinitions)

            expect(formatted.is_active).toBe(null)
            expect(formatted.age).toBe(null)
            // Empty string for String type should return null (as per formatValueToType logic)
            expect(formatted.name).toBe(null)
        })

        it('should handle complex nested data', () => {
            const fieldData = {
                metadata: '{"key":"value"}',
                tags: '[1,2,3]',
            }

            const fieldDefinitions = [
                { name: 'metadata', field_type: 'Object' as FieldType },
                { name: 'tags', field_type: 'Array' as FieldType },
            ]

            const formatted = formatFieldData(fieldData, fieldDefinitions)

            expect(formatted.metadata).toEqual({ key: 'value' })
            expect(formatted.tags).toEqual([1, 2, 3])
        })
    })
})
