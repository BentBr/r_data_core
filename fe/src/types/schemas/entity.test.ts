import { describe, it, expect } from 'vitest'
import { FieldDefinitionSchema, EntityDefinitionSchema } from './entity'

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
})
