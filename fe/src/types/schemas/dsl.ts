import { z } from 'zod'
import { ApiResponseSchema } from './base'

export const CsvOptionsSchema = z.object({
    has_header: z.boolean().optional(),
    delimiter: z.string().min(1).max(1).optional(),
    escape: z.string().min(1).max(1).optional(),
    quote: z.string().min(1).max(1).optional(),
})

// Auth configuration schemas
const KeyLocationSchema = z.enum(['header', 'body'])

const AuthConfigSchema = z.discriminatedUnion('type', [
    z.object({ type: z.literal('none') }),
    z.object({
        type: z.literal('api_key'),
        key: z.string(),
        header_name: z.string().default('X-API-Key'),
    }),
    z.object({
        type: z.literal('basic_auth'),
        username: z.string(),
        password: z.string(),
    }),
    z.object({
        type: z.literal('pre_shared_key'),
        key: z.string(),
        location: KeyLocationSchema,
        field_name: z.string(),
    }),
    z.object({
        type: z.literal('entity_jwt'),
        required_claims: z.record(z.string(), z.any()).optional(),
    }),
])

// Source configuration
const SourceConfigSchema = z.object({
    source_type: z.string(), // "uri", "file", "api", "sftp", etc.
    config: z.record(z.string(), z.unknown()), // Source-specific config (e.g., { uri: "..." } or { endpoint: "..." })
    auth: AuthConfigSchema.optional(),
})

// Format configuration
const FormatConfigSchema = z.object({
    format_type: z.string(), // "csv", "json", "xml", etc.
    options: z.record(z.string(), z.unknown()).optional(), // Format-specific options
})

// New Format-based FromDef
export const DslFromFormatSchema = z.object({
    type: z.literal('format'),
    source: SourceConfigSchema,
    format: FormatConfigSchema,
    mapping: z.record(z.string(), z.string()),
})

export const DslFromJsonSchema = z.object({
    type: z.literal('json'),
    uri: z.string(),
    mapping: z.record(z.string(), z.string()),
})

export const DslEntityFilterSchema = z.object({
    field: z.string(),
    operator: z.string().optional(),
    value: z.string(),
})

export const DslFromEntitySchema = z.object({
    type: z.literal('entity'),
    entity_definition: z.string(),
    filter: DslEntityFilterSchema.optional(),
    mapping: z.record(z.string(), z.string()),
})

export const DslFromPreviousStepSchema = z.object({
    type: z.literal('previous_step'),
    mapping: z.record(z.string(), z.string()),
})

export const DslFromTriggerSchema = z.object({
    type: z.literal('trigger'),
    mapping: z.record(z.string(), z.string()),
})

export const DslFromSchema = z.discriminatedUnion('type', [
    DslFromFormatSchema, // New format-based structure
    DslFromEntitySchema,
    DslFromPreviousStepSchema, // Step chaining support
    DslFromTriggerSchema, // Webhook trigger support
])

// Destination configuration
const DestinationConfigSchema = z.object({
    destination_type: z.string(), // "uri", "file", "sftp", etc.
    config: z.record(z.string(), z.unknown()), // Destination-specific config (e.g., { uri: "..." })
    auth: AuthConfigSchema.optional(),
})

// HTTP method enum
const HttpMethodSchema = z.enum(['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'])

// Output mode
const OutputModeSchema = z.discriminatedUnion('mode', [
    z.object({ mode: z.literal('download') }),
    z.object({ mode: z.literal('api') }),
    z.object({
        mode: z.literal('push'),
        destination: DestinationConfigSchema,
        method: HttpMethodSchema.optional(),
    }),
])

// New Format-based ToDef
export const DslToFormatSchema = z.object({
    type: z.literal('format'),
    output: OutputModeSchema,
    format: FormatConfigSchema,
    mapping: z.record(z.string(), z.string()),
})

export const DslToEntitySchema = z.object({
    type: z.literal('entity'),
    entity_definition: z.string(),
    path: z.string().optional(), // Optional - derived from parent_uuid if not set
    mode: z.enum(['create', 'update', 'create_or_update']),
    identify: DslEntityFilterSchema.optional(),
    update_key: z.string().optional(),
    mapping: z.record(z.string(), z.string()),
})

export const DslToNextStepSchema = z.object({
    type: z.literal('next_step'),
    mapping: z.record(z.string(), z.string()),
})

export const DslToSchema = z.discriminatedUnion('type', [
    DslToFormatSchema,
    DslToEntitySchema,
    DslToNextStepSchema,
])

export const DslOperandFieldSchema = z.object({
    kind: z.literal('field'),
    field: z.string(),
})
export const DslOperandConstSchema = z.object({
    kind: z.literal('const'),
    value: z.number(),
})
export const DslOperandExternalSchema = z.object({
    kind: z.literal('external_entity_field'),
    entity_definition: z.string(),
    filter: DslEntityFilterSchema,
    field: z.string(),
})
export const DslOperandSchema = z.discriminatedUnion('kind', [
    DslOperandFieldSchema,
    DslOperandConstSchema,
    DslOperandExternalSchema,
])

// String operands for concat
export const DslStringOperandFieldSchema = z.object({
    kind: z.literal('field'),
    field: z.string(),
})
export const DslStringOperandConstSchema = z.object({
    kind: z.literal('const_string'),
    value: z.string(),
})
export const DslStringOperandSchema = z.discriminatedUnion('kind', [
    DslStringOperandFieldSchema,
    DslStringOperandConstSchema,
])

export const DslTransformArithmeticSchema = z.object({
    type: z.literal('arithmetic'),
    target: z.string(),
    left: DslOperandSchema,
    op: z.enum(['add', 'sub', 'mul', 'div']),
    right: DslOperandSchema,
})
export const DslTransformNoneSchema = z.object({
    type: z.literal('none'),
})
export const DslTransformConcatSchema = z.object({
    type: z.literal('concat'),
    target: z.string(),
    left: DslStringOperandSchema,
    separator: z.string().optional(),
    right: DslStringOperandSchema,
})
export const DslTransformResolveEntityPathSchema = z.object({
    type: z.literal('resolve_entity_path'),
    target_path: z.string(),
    target_uuid: z.string().optional(),
    entity_type: z.string(),
    filters: z.record(z.string(), DslStringOperandSchema),
    value_transforms: z.record(z.string(), z.string()).optional(),
    fallback_path: z.string().optional(),
})
export const DslTransformBuildPathSchema = z.object({
    type: z.literal('build_path'),
    target: z.string(),
    template: z.string(),
    separator: z.string().optional(),
    field_transforms: z.record(z.string(), z.string()).optional(),
})
export const DslTransformGetOrCreateEntitySchema = z.object({
    type: z.literal('get_or_create_entity'),
    target_path: z.string(),
    target_uuid: z.string().optional(),
    entity_type: z.string(),
    path_template: z.string(),
    create_field_data: z.record(z.string(), DslStringOperandSchema).optional(),
    path_separator: z.string().optional(),
})
export const DslTransformAuthenticateSchema = z.object({
    type: z.literal('authenticate'),
    entity_type: z.string(),
    identifier_field: z.string(),
    password_field: z.string(),
    input_identifier: z.string(),
    input_password: z.string(),
    target_token: z.string(),
    extra_claims: z.record(z.string(), z.string()).optional(),
    token_expiry_seconds: z.number().optional(),
})
export const DslTransformSchema = z.discriminatedUnion('type', [
    DslTransformNoneSchema,
    DslTransformArithmeticSchema,
    DslTransformConcatSchema,
    DslTransformResolveEntityPathSchema,
    DslTransformBuildPathSchema,
    DslTransformGetOrCreateEntitySchema,
    DslTransformAuthenticateSchema,
])

export const DslStepSchema = z.object({
    from: DslFromSchema,
    to: DslToSchema,
    transform: DslTransformSchema,
})

export const DslValidateRequestSchema = z.object({
    steps: z.array(DslStepSchema).min(1),
})

export const DslValidateResponseSchema = z.object({
    valid: z.boolean(),
})

export const DslFieldSpecSchema = z.object({
    name: z.string(),
    type: z.string(),
    required: z.boolean(),
    options: z.array(z.string()).optional(),
})

export const DslTypeSpecSchema = z.object({
    type: z.string(),
    fields: z.array(DslFieldSpecSchema),
})

export const DslOptionsResponseSchema = z.object({
    types: z.array(DslTypeSpecSchema),
    examples: z.array(z.unknown()).optional(),
})

export type DslStep = z.infer<typeof DslStepSchema>
export type DslValidateRequest = z.infer<typeof DslValidateRequestSchema>
export type DslValidateResponse = z.infer<typeof DslValidateResponseSchema>
export type DslOptionsResponse = z.infer<typeof DslOptionsResponseSchema>

// Export component types for DSL editors
export type FromDef = z.infer<typeof DslFromSchema>
export type ToDef = z.infer<typeof DslToSchema>
export type Transform = z.infer<typeof DslTransformSchema>
export type OutputMode = z.infer<typeof OutputModeSchema>
export type HttpMethod = z.infer<typeof HttpMethodSchema>
export type Operand = z.infer<typeof DslOperandSchema>
export type StringOperand = z.infer<typeof DslStringOperandSchema>
export type AuthConfig = z.infer<typeof AuthConfigSchema>
export type SourceConfig = z.infer<typeof SourceConfigSchema>
export type FormatConfig = z.infer<typeof FormatConfigSchema>
export type DestinationConfig = z.infer<typeof DestinationConfigSchema>

// Re-export ApiResponseSchema for consumers that need it alongside DSL
export { ApiResponseSchema }
