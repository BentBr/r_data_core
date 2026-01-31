import { describe, it, expect } from 'vitest'
import { FieldDefinitionSchema, EntityDefinitionSchema, DynamicEntitySchema } from './entity'

describe('Entity Schemas', () => {
    describe('FieldDefinitionSchema', () => {
        it('should accept null values for constraints, ui_settings, and default_value', () => {
            const field = {
                name: 'test_field',
                display_name: 'Test Field',
                field_type: 'String' as const,
                required: true,
                indexed: false,
                filterable: false,
                default_value: null,
                constraints: null,
                ui_settings: null,
            }

            const result = FieldDefinitionSchema.safeParse(field)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.default_value).toBeNull()
                expect(result.data.constraints).toBeNull()
                expect(result.data.ui_settings).toBeNull()
            }
        })

        it('should accept undefined values for constraints, ui_settings, and default_value', () => {
            const field = {
                name: 'test_field',
                display_name: 'Test Field',
                field_type: 'String' as const,
                required: true,
                indexed: false,
                filterable: false,
                default_value: undefined,
                constraints: undefined,
                ui_settings: undefined,
            }

            const result = FieldDefinitionSchema.safeParse(field)
            expect(result.success).toBe(true)
        })

        it('should accept record values for constraints and ui_settings', () => {
            const field = {
                name: 'test_field',
                display_name: 'Test Field',
                field_type: 'String' as const,
                required: true,
                indexed: false,
                filterable: false,
                default_value: 'test',
                constraints: { min: 1, max: 100 },
                ui_settings: { placeholder: 'Enter value' },
            }

            const result = FieldDefinitionSchema.safeParse(field)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.constraints).toEqual({ min: 1, max: 100 })
                expect(result.data.ui_settings).toEqual({ placeholder: 'Enter value' })
            }
        })

        it('should validate real API response with null values', () => {
            // This matches the actual API response structure from the user's error
            const apiField = {
                name: 'username',
                display_name: 'Username',
                field_type: 'String' as const,
                description: '',
                required: true,
                indexed: false,
                filterable: true,
                default_value: null,
                constraints: null,
                ui_settings: {
                    placeholder: null,
                    help_text: null,
                    hide_in_lists: null,
                    width: null,
                    order: null,
                    group: null,
                    css_class: null,
                    wysiwyg_toolbar: null,
                    input_type: null,
                },
            }

            const result = FieldDefinitionSchema.safeParse(apiField)
            expect(result.success).toBe(true)
        })
    })

    describe('EntityDefinitionSchema', () => {
        it('should validate entity definition with null field values', () => {
            const entityDef = {
                uuid: '019a9766-20bb-7533-a3fc-8bc07fa4491a',
                entity_type: 'customer',
                display_name: 'Customer',
                description: 'Test customer',
                group_name: '',
                allow_children: false,
                icon: 'user',
                fields: [
                    {
                        name: 'username',
                        display_name: 'Username',
                        field_type: 'String' as const,
                        description: '',
                        required: true,
                        indexed: false,
                        filterable: true,
                        default_value: null,
                        constraints: null,
                        ui_settings: null,
                    },
                ],
                published: true,
                created_at: '2025-11-18T14:37:24.027863Z',
                updated_at: '2025-11-19T14:58:25.727591Z',
            }

            const result = EntityDefinitionSchema.safeParse(entityDef)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.fields[0].constraints).toBeNull()
                expect(result.data.fields[0].ui_settings).toBeNull()
                expect(result.data.fields[0].default_value).toBeNull()
            }
        })

        it('should validate paginated response with null constraints', () => {
            // This matches the actual API response from the user's error
            const apiResponse = {
                status: 'Success',
                message: 'Operation completed successfully',
                data: [
                    {
                        uuid: '019a9766-20bb-7533-a3fc-8bc07fa4491a',
                        entity_type: 'customer',
                        display_name: 'Customer',
                        description: 'Toller Kunde\nmeta info',
                        group_name: '',
                        allow_children: false,
                        icon: 'user',
                        fields: [
                            {
                                name: 'username',
                                display_name: 'Username',
                                field_type: 'String' as const,
                                description: '',
                                required: true,
                                indexed: false,
                                filterable: true,
                                default_value: null,
                                constraints: null,
                                ui_settings: {
                                    placeholder: null,
                                    help_text: null,
                                    hide_in_lists: null,
                                    width: null,
                                    order: null,
                                    group: null,
                                    css_class: null,
                                    wysiwyg_toolbar: null,
                                    input_type: null,
                                },
                            },
                        ],
                        published: true,
                        created_at: '2025-11-18T14:37:24.027863Z',
                        updated_at: '2025-11-19T14:58:25.727591Z',
                    },
                ],
            }

            // Validate each entity definition in the response
            for (const entityDef of apiResponse.data) {
                const result = EntityDefinitionSchema.safeParse(entityDef)
                expect(result.success).toBe(true)
                if (result.success) {
                    for (const field of result.data.fields) {
                        // Should accept null for constraints
                        expect(field.constraints).toBeNull()
                    }
                }
            }
        })
    })

    describe('FieldDefinitionSchema field_type variants', () => {
        it('should accept Json field type', () => {
            const field = {
                name: 'metadata',
                display_name: 'Metadata',
                field_type: 'Json' as const,
                required: false,
                indexed: false,
                filterable: false,
            }

            const result = FieldDefinitionSchema.safeParse(field)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.field_type).toBe('Json')
            }
        })

        it('should accept Object field type (distinct from Json)', () => {
            const field = {
                name: 'settings',
                display_name: 'Settings',
                field_type: 'Object' as const,
                required: false,
                indexed: false,
                filterable: false,
            }

            const result = FieldDefinitionSchema.safeParse(field)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.field_type).toBe('Object')
            }
        })

        it('should accept Array field type', () => {
            const field = {
                name: 'tags',
                display_name: 'Tags',
                field_type: 'Array' as const,
                required: false,
                indexed: false,
                filterable: false,
            }

            const result = FieldDefinitionSchema.safeParse(field)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.field_type).toBe('Array')
            }
        })

        it('should validate entity definition with Json field for API arrays', () => {
            // This is the actual use case: statistics submission with JSON arrays
            const entityDef = {
                uuid: '019a9766-20bb-7533-a3fc-8bc07fa4491a',
                entity_type: 'StatisticSubmission',
                display_name: 'Statistic Submission',
                description: 'Statistics from instances',
                allow_children: false,
                fields: [
                    {
                        name: 'cors_origins',
                        display_name: 'CORS Origins',
                        field_type: 'Json' as const,
                        description: 'Array of allowed CORS origins',
                        required: false,
                        indexed: false,
                        filterable: false,
                        constraints: null,
                        ui_settings: null,
                    },
                    {
                        name: 'entities_per_definition',
                        display_name: 'Entities Per Definition',
                        field_type: 'Json' as const,
                        description: 'Array of entity type counts',
                        required: false,
                        indexed: false,
                        filterable: false,
                        constraints: null,
                        ui_settings: null,
                    },
                    {
                        name: 'entity_definitions',
                        display_name: 'Entity Definitions',
                        field_type: 'Json' as const,
                        description: 'Object with entity definition info',
                        required: false,
                        indexed: false,
                        filterable: false,
                        constraints: null,
                        ui_settings: null,
                    },
                ],
                published: true,
            }

            const result = EntityDefinitionSchema.safeParse(entityDef)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.fields[0].field_type).toBe('Json')
                expect(result.data.fields[1].field_type).toBe('Json')
                expect(result.data.fields[2].field_type).toBe('Json')
            }
        })

        it('should reject invalid field type', () => {
            const field = {
                name: 'test_field',
                display_name: 'Test Field',
                field_type: 'InvalidType',
                required: false,
                indexed: false,
                filterable: false,
            }

            const result = FieldDefinitionSchema.safeParse(field)
            expect(result.success).toBe(false)
        })
    })

    describe('DynamicEntitySchema', () => {
        it('should accept entity without children_count', () => {
            const entity = {
                entity_type: 'Customer',
                field_data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    name: 'Test Customer',
                },
            }

            const result = DynamicEntitySchema.safeParse(entity)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.children_count).toBeUndefined()
            }
        })

        it('should accept entity with children_count as number', () => {
            const entity = {
                entity_type: 'Customer',
                field_data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    name: 'Test Customer',
                },
                children_count: 5,
            }

            const result = DynamicEntitySchema.safeParse(entity)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.children_count).toBe(5)
            }
        })

        it('should accept entity with children_count as zero', () => {
            const entity = {
                entity_type: 'Customer',
                field_data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    name: 'Test Customer',
                },
                children_count: 0,
            }

            const result = DynamicEntitySchema.safeParse(entity)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.children_count).toBe(0)
            }
        })

        it('should accept entity with children_count as null', () => {
            const entity = {
                entity_type: 'Customer',
                field_data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    name: 'Test Customer',
                },
                children_count: null,
            }

            const result = DynamicEntitySchema.safeParse(entity)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.children_count).toBeNull()
            }
        })

        it('should validate API response with children_count', () => {
            // This matches the actual API response structure
            const apiResponse = {
                entity_type: 'Customer',
                field_data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    name: 'Test Customer',
                    created_at: '2024-01-01T00:00:00Z',
                    updated_at: '2024-01-01T00:00:00Z',
                },
                children_count: 10,
            }

            const result = DynamicEntitySchema.safeParse(apiResponse)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.entity_type).toBe('Customer')
                expect(result.data.children_count).toBe(10)
            }
        })
    })
})
