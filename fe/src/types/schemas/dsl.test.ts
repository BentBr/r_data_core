import { describe, it, expect } from 'vitest'
import {
    DslTransformAuthenticateSchema,
    DslTransformSchema,
    DslStepSchema,
    DslValidateRequestSchema,
    DslTransformNoneSchema,
    DslTransformArithmeticSchema,
    DslTransformConcatSchema,
    DslTransformBuildPathSchema,
    DslTransformResolveEntityPathSchema,
    DslTransformGetOrCreateEntitySchema,
    DslFromSchema,
    DslToSchema,
    CsvOptionsSchema,
} from './dsl'

describe('DSL Schemas', () => {
    describe('DslTransformAuthenticateSchema', () => {
        it('should accept a valid authenticate transform with all fields', () => {
            const auth = {
                type: 'authenticate',
                entity_type: 'user',
                identifier_field: 'email',
                password_field: 'password',
                input_identifier: 'identifier',
                input_password: 'password',
                target_token: 'token',
                extra_claims: { role: 'role' },
                token_expiry_seconds: 3600,
            }

            const result = DslTransformAuthenticateSchema.safeParse(auth)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.entity_type).toBe('user')
                expect(result.data.extra_claims).toEqual({ role: 'role' })
                expect(result.data.token_expiry_seconds).toBe(3600)
            }
        })

        it('should accept authenticate transform without optional fields', () => {
            const auth = {
                type: 'authenticate',
                entity_type: 'user',
                identifier_field: 'email',
                password_field: 'password',
                input_identifier: 'identifier',
                input_password: 'password',
                target_token: 'token',
            }

            const result = DslTransformAuthenticateSchema.safeParse(auth)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.extra_claims).toBeUndefined()
                expect(result.data.token_expiry_seconds).toBeUndefined()
            }
        })

        it('should accept authenticate transform with empty extra_claims', () => {
            const auth = {
                type: 'authenticate',
                entity_type: 'user',
                identifier_field: 'email',
                password_field: 'password',
                input_identifier: 'identifier',
                input_password: 'password',
                target_token: 'token',
                extra_claims: {},
            }

            const result = DslTransformAuthenticateSchema.safeParse(auth)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.extra_claims).toEqual({})
            }
        })

        it('should accept authenticate transform with multiple extra_claims', () => {
            const auth = {
                type: 'authenticate',
                entity_type: 'user',
                identifier_field: 'email',
                password_field: 'password',
                input_identifier: 'identifier',
                input_password: 'password',
                target_token: 'token',
                extra_claims: { role: 'role', department: 'department', level: 'access_level' },
            }

            const result = DslTransformAuthenticateSchema.safeParse(auth)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(Object.keys(result.data.extra_claims!)).toHaveLength(3)
            }
        })

        it('should reject authenticate transform missing required fields', () => {
            const incomplete = {
                type: 'authenticate',
                entity_type: 'user',
                // missing identifier_field, password_field, etc.
            }

            const result = DslTransformAuthenticateSchema.safeParse(incomplete)
            expect(result.success).toBe(false)
        })

        it('should reject non-string extra_claims values', () => {
            const auth = {
                type: 'authenticate',
                entity_type: 'user',
                identifier_field: 'email',
                password_field: 'password',
                input_identifier: 'identifier',
                input_password: 'password',
                target_token: 'token',
                extra_claims: { role: 123 },
            }

            const result = DslTransformAuthenticateSchema.safeParse(auth)
            expect(result.success).toBe(false)
        })

        it('should reject non-number token_expiry_seconds', () => {
            const auth = {
                type: 'authenticate',
                entity_type: 'user',
                identifier_field: 'email',
                password_field: 'password',
                input_identifier: 'identifier',
                input_password: 'password',
                target_token: 'token',
                token_expiry_seconds: 'not_a_number',
            }

            const result = DslTransformAuthenticateSchema.safeParse(auth)
            expect(result.success).toBe(false)
        })
    })

    describe('DslTransformSchema (discriminated union)', () => {
        it('should accept none transform', () => {
            const result = DslTransformSchema.safeParse({ type: 'none' })
            expect(result.success).toBe(true)
        })

        it('should accept arithmetic transform', () => {
            const result = DslTransformSchema.safeParse({
                type: 'arithmetic',
                target: 'total',
                left: { kind: 'field', field: 'price' },
                op: 'mul',
                right: { kind: 'const', value: 1.19 },
            })
            expect(result.success).toBe(true)
        })

        it('should accept concat transform', () => {
            const result = DslTransformSchema.safeParse({
                type: 'concat',
                target: 'full_name',
                left: { kind: 'field', field: 'first_name' },
                separator: ' ',
                right: { kind: 'field', field: 'last_name' },
            })
            expect(result.success).toBe(true)
        })

        it('should accept build_path transform', () => {
            const result = DslTransformSchema.safeParse({
                type: 'build_path',
                target: 'path',
                template: '/{region}/{city}',
            })
            expect(result.success).toBe(true)
        })

        it('should accept resolve_entity_path transform', () => {
            const result = DslTransformSchema.safeParse({
                type: 'resolve_entity_path',
                target_path: 'resolved_path',
                entity_type: 'location',
                filters: {
                    name: { kind: 'field', field: 'location_name' },
                },
            })
            expect(result.success).toBe(true)
        })

        it('should accept get_or_create_entity transform', () => {
            const result = DslTransformSchema.safeParse({
                type: 'get_or_create_entity',
                target_path: 'entity_path',
                entity_type: 'category',
                path_template: '/{category_name}',
            })
            expect(result.success).toBe(true)
        })

        it('should accept authenticate transform via union', () => {
            const result = DslTransformSchema.safeParse({
                type: 'authenticate',
                entity_type: 'user',
                identifier_field: 'email',
                password_field: 'password',
                input_identifier: 'identifier',
                input_password: 'password',
                target_token: 'token',
            })
            expect(result.success).toBe(true)
        })

        it('should reject unknown transform type', () => {
            const result = DslTransformSchema.safeParse({
                type: 'unknown_transform',
                foo: 'bar',
            })
            expect(result.success).toBe(false)
        })
    })

    describe('DslStepSchema', () => {
        it('should validate a complete authenticate login step', () => {
            const step = {
                from: {
                    type: 'format',
                    source: { source_type: 'api', config: {} },
                    format: { format_type: 'json', options: {} },
                    mapping: { email: 'identifier', password: 'password' },
                },
                transform: {
                    type: 'authenticate',
                    entity_type: 'user',
                    identifier_field: 'email',
                    password_field: 'password',
                    input_identifier: 'identifier',
                    input_password: 'password',
                    target_token: 'token',
                    extra_claims: { role: 'role' },
                    token_expiry_seconds: 3600,
                },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'json', options: {} },
                    mapping: { token: 'access_token' },
                },
            }

            const result = DslStepSchema.safeParse(step)
            expect(result.success).toBe(true)
            if (result.success) {
                expect(result.data.transform.type).toBe('authenticate')
            }
        })

        it('should validate a step with entity from and entity to', () => {
            const step = {
                from: {
                    type: 'entity',
                    entity_definition: 'customer',
                    mapping: { name: 'customer_name' },
                },
                transform: { type: 'none' },
                to: {
                    type: 'entity',
                    entity_definition: 'report',
                    mode: 'create',
                    mapping: { customer_name: 'source_name' },
                },
            }

            const result = DslStepSchema.safeParse(step)
            expect(result.success).toBe(true)
        })

        it('should validate a step with previous_step from and next_step to', () => {
            const step = {
                from: {
                    type: 'previous_step',
                    mapping: { result: 'input' },
                },
                transform: {
                    type: 'arithmetic',
                    target: 'total',
                    left: { kind: 'field', field: 'input' },
                    op: 'add',
                    right: { kind: 'const', value: 10 },
                },
                to: {
                    type: 'next_step',
                    mapping: { total: 'next_input' },
                },
            }

            const result = DslStepSchema.safeParse(step)
            expect(result.success).toBe(true)
        })

        it('should validate a step with trigger from', () => {
            const step = {
                from: {
                    type: 'trigger',
                    mapping: {},
                },
                transform: { type: 'none' },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'json' },
                    mapping: {},
                },
            }

            const result = DslStepSchema.safeParse(step)
            expect(result.success).toBe(true)
        })

        it('should reject a step missing the transform field', () => {
            const step = {
                from: {
                    type: 'format',
                    source: { source_type: 'api', config: {} },
                    format: { format_type: 'json' },
                    mapping: {},
                },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'json' },
                    mapping: {},
                },
            }

            const result = DslStepSchema.safeParse(step)
            expect(result.success).toBe(false)
        })
    })

    describe('DslValidateRequestSchema', () => {
        it('should accept a valid multi-step request', () => {
            const request = {
                steps: [
                    {
                        from: {
                            type: 'format',
                            source: { source_type: 'api', config: {} },
                            format: { format_type: 'json' },
                            mapping: { email: 'identifier', password: 'password' },
                        },
                        transform: {
                            type: 'authenticate',
                            entity_type: 'user',
                            identifier_field: 'email',
                            password_field: 'password',
                            input_identifier: 'identifier',
                            input_password: 'password',
                            target_token: 'token',
                        },
                        to: {
                            type: 'format',
                            output: { mode: 'api' },
                            format: { format_type: 'json' },
                            mapping: { token: 'access_token' },
                        },
                    },
                ],
            }

            const result = DslValidateRequestSchema.safeParse(request)
            expect(result.success).toBe(true)
        })

        it('should reject an empty steps array', () => {
            const result = DslValidateRequestSchema.safeParse({ steps: [] })
            expect(result.success).toBe(false)
        })
    })

    describe('DslFromSchema', () => {
        it('should accept format from with auth config', () => {
            const from = {
                type: 'format',
                source: {
                    source_type: 'uri',
                    config: { uri: 'https://example.com/data.csv' },
                    auth: { type: 'api_key', key: 'secret123', header_name: 'X-API-Key' },
                },
                format: { format_type: 'csv', options: { has_header: true, delimiter: ',' } },
                mapping: { col1: 'field1' },
            }

            const result = DslFromSchema.safeParse(from)
            expect(result.success).toBe(true)
        })

        it('should accept format from with entity_jwt auth', () => {
            const from = {
                type: 'format',
                source: {
                    source_type: 'api',
                    config: {},
                    auth: {
                        type: 'entity_jwt',
                        required_claims: { entity_type: 'user', 'extra.role': 'admin' },
                    },
                },
                format: { format_type: 'json' },
                mapping: {},
            }

            const result = DslFromSchema.safeParse(from)
            expect(result.success).toBe(true)
        })

        it('should accept format from without auth', () => {
            const from = {
                type: 'format',
                source: { source_type: 'api', config: {} },
                format: { format_type: 'json' },
                mapping: {},
            }

            const result = DslFromSchema.safeParse(from)
            expect(result.success).toBe(true)
        })
    })

    describe('DslToSchema', () => {
        it('should accept format to with push output', () => {
            const to = {
                type: 'format',
                output: {
                    mode: 'push',
                    destination: {
                        destination_type: 'uri',
                        config: { uri: 'https://example.com/webhook' },
                    },
                    method: 'POST',
                },
                format: { format_type: 'json' },
                mapping: { field1: 'out1' },
            }

            const result = DslToSchema.safeParse(to)
            expect(result.success).toBe(true)
        })

        it('should accept entity to with create_or_update mode', () => {
            const to = {
                type: 'entity',
                entity_definition: 'customer',
                mode: 'create_or_update',
                identify: { field: 'email', value: 'test@example.com' },
                mapping: { name: 'customer_name' },
            }

            const result = DslToSchema.safeParse(to)
            expect(result.success).toBe(true)
        })

        it('should reject entity to with invalid mode', () => {
            const to = {
                type: 'entity',
                entity_definition: 'customer',
                mode: 'delete',
                mapping: {},
            }

            const result = DslToSchema.safeParse(to)
            expect(result.success).toBe(false)
        })
    })

    describe('CsvOptionsSchema', () => {
        it('should accept valid CSV options', () => {
            const result = CsvOptionsSchema.safeParse({
                has_header: true,
                delimiter: ',',
                escape: '\\',
                quote: '"',
            })
            expect(result.success).toBe(true)
        })

        it('should accept empty CSV options (all optional)', () => {
            const result = CsvOptionsSchema.safeParse({})
            expect(result.success).toBe(true)
        })

        it('should reject multi-character delimiter', () => {
            const result = CsvOptionsSchema.safeParse({ delimiter: '||' })
            expect(result.success).toBe(false)
        })
    })

    describe('Individual transform schemas', () => {
        it('DslTransformNoneSchema accepts minimal none', () => {
            const result = DslTransformNoneSchema.safeParse({ type: 'none' })
            expect(result.success).toBe(true)
        })

        it('DslTransformArithmeticSchema validates all ops', () => {
            for (const op of ['add', 'sub', 'mul', 'div']) {
                const result = DslTransformArithmeticSchema.safeParse({
                    type: 'arithmetic',
                    target: 'result',
                    left: { kind: 'field', field: 'a' },
                    op,
                    right: { kind: 'const', value: 5 },
                })
                expect(result.success).toBe(true)
            }
        })

        it('DslTransformArithmeticSchema rejects invalid op', () => {
            const result = DslTransformArithmeticSchema.safeParse({
                type: 'arithmetic',
                target: 'result',
                left: { kind: 'field', field: 'a' },
                op: 'modulo',
                right: { kind: 'const', value: 5 },
            })
            expect(result.success).toBe(false)
        })

        it('DslTransformConcatSchema accepts const_string operands', () => {
            const result = DslTransformConcatSchema.safeParse({
                type: 'concat',
                target: 'greeting',
                left: { kind: 'const_string', value: 'Hello ' },
                right: { kind: 'field', field: 'name' },
            })
            expect(result.success).toBe(true)
        })

        it('DslTransformBuildPathSchema accepts optional fields', () => {
            const result = DslTransformBuildPathSchema.safeParse({
                type: 'build_path',
                target: 'path',
                template: '/{a}/{b}',
                separator: '/',
                field_transforms: { a: 'lowercase', b: 'slug' },
            })
            expect(result.success).toBe(true)
        })

        it('DslTransformResolveEntityPathSchema accepts full config', () => {
            const result = DslTransformResolveEntityPathSchema.safeParse({
                type: 'resolve_entity_path',
                target_path: 'resolved',
                target_uuid: 'parent_uuid',
                entity_type: 'location',
                filters: { name: { kind: 'const_string', value: 'Berlin' } },
                value_transforms: { name: 'lowercase' },
                fallback_path: '/default',
            })
            expect(result.success).toBe(true)
        })

        it('DslTransformGetOrCreateEntitySchema accepts create_field_data', () => {
            const result = DslTransformGetOrCreateEntitySchema.safeParse({
                type: 'get_or_create_entity',
                target_path: 'entity_path',
                target_uuid: 'entity_uuid',
                entity_type: 'category',
                path_template: '/{category_name}',
                create_field_data: {
                    display_name: { kind: 'field', field: 'category_name' },
                },
                path_separator: '/',
            })
            expect(result.success).toBe(true)
        })
    })

    describe('Full workflow JSON examples', () => {
        it('should validate the authenticate login workflow example', () => {
            const workflow = {
                steps: [
                    {
                        from: {
                            type: 'format',
                            source: { source_type: 'api', config: {} },
                            format: { format_type: 'json', options: {} },
                            mapping: { email: 'identifier', password: 'password' },
                        },
                        transform: {
                            type: 'authenticate',
                            entity_type: 'user',
                            identifier_field: 'email',
                            password_field: 'password',
                            input_identifier: 'identifier',
                            input_password: 'password',
                            target_token: 'token',
                            extra_claims: { role: 'role' },
                            token_expiry_seconds: 3600,
                        },
                        to: {
                            type: 'format',
                            output: { mode: 'api' },
                            format: { format_type: 'json', options: {} },
                            mapping: { token: 'access_token' },
                        },
                    },
                ],
            }

            const result = DslValidateRequestSchema.safeParse(workflow)
            expect(result.success).toBe(true)
        })

        it('should validate a JWT-protected entity workflow example', () => {
            const workflow = {
                steps: [
                    {
                        from: {
                            type: 'format',
                            source: {
                                source_type: 'api',
                                config: {},
                                auth: {
                                    type: 'entity_jwt',
                                    required_claims: { entity_type: 'user' },
                                },
                            },
                            format: { format_type: 'json' },
                            mapping: { name: 'name', value: 'value' },
                        },
                        transform: { type: 'none' },
                        to: {
                            type: 'entity',
                            entity_definition: 'profile',
                            mode: 'create_or_update',
                            identify: { field: 'name', value: '' },
                            mapping: { name: 'name', value: 'value' },
                        },
                    },
                ],
            }

            const result = DslValidateRequestSchema.safeParse(workflow)
            expect(result.success).toBe(true)
        })
    })
})
