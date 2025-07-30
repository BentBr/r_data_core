import { z } from 'zod'

// Base schemas for common patterns - handle both traditional UUID and UUIDv7 formats
const UuidSchema = z.string().refine(
    (val) => {
        // UUID format: 8-4-4-4-12 hexadecimal (covers UUID v1-v7)
        const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
        return uuidRegex.test(val)
    },
    {
        message: "Invalid UUID format",
    }
)
// More flexible timestamp schema to handle backend nanosecond precision
const TimestampSchema = z.string().refine(
    (val) => {
        // Allow ISO 8601 format with varying precision
        const isoRegex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z?$/
        return isoRegex.test(val) && !isNaN(Date.parse(val))
    },
    {
        message: "Invalid timestamp format, expected ISO 8601",
    }
)

// API Response wrapper schema
export const ApiResponseSchema = <T extends z.ZodTypeAny>(dataSchema: T) =>
    z.object({
        status: z.enum(['Success', 'Error']),
        message: z.string(),
        data: dataSchema.optional(),
        meta: z
            .object({
                pagination: z
                    .object({
                        total: z.number(),
                        page: z.number(),
                        per_page: z.number(),
                        total_pages: z.number(),
                        has_previous: z.boolean(),
                        has_next: z.boolean(),
                    })
                    .optional(),
                request_id: UuidSchema.optional(),
                timestamp: TimestampSchema.optional(),
                custom: z.any().optional(),
            })
            .nullish(), // Allow null, undefined, or the object
    })

// Auth schemas
export const LoginRequestSchema = z.object({
    username: z.string().min(3),
    password: z.string().min(8),
})

export const LoginResponseSchema = z.object({
    token: z.string(),
    user_uuid: UuidSchema,
    username: z.string(),
    role: z.string(),
    expires_at: TimestampSchema,
})

// Field Definition schema
export const FieldDefinitionSchema = z.object({
    field_name: z.string(),
    display_name: z.string(),
    field_type: z.enum([
        'String',
        'Text',
        'Wysiwyg',
        'Integer',
        'Float',
        'Boolean',
        'Date',
        'DateTime',
        'Object',
        'Array',
        'UUID',
        'ManyToOne',
        'ManyToMany',
        'Select',
        'MultiSelect',
        'Image',
        'File',
    ]),
    is_required: z.boolean(),
    constraints: z.record(z.string(), z.any()).optional(),
    ui_options: z.record(z.string(), z.any()).optional(),
})

// Class Definition schema
export const ClassDefinitionSchema = z.object({
    uuid: UuidSchema,
    entity_type: z.string(),
    display_name: z.string(),
    description: z.string().optional(),
    field_definitions: z.array(FieldDefinitionSchema),
    created_at: TimestampSchema,
    updated_at: TimestampSchema,
    created_by: UuidSchema,
    updated_by: UuidSchema.optional(),
    version: z.number().int().positive(),
})

// Dynamic Entity schema
export const DynamicEntitySchema = z.object({
    uuid: UuidSchema,
    entity_type: z.string(),
    data: z.record(z.string(), z.any()),
    created_at: TimestampSchema,
    updated_at: TimestampSchema,
})

// API Key schemas
export const ApiKeySchema = z.object({
    uuid: UuidSchema,
    name: z.string(),
    description: z.string().optional(),
    is_active: z.boolean(),
    created_at: TimestampSchema,
    expires_at: TimestampSchema.optional(),
    last_used_at: TimestampSchema.optional(),
    created_by: UuidSchema,
    user_uuid: UuidSchema,
    published: z.boolean(),
})

export const CreateApiKeyRequestSchema = z.object({
    name: z.string().min(1),
    description: z.string().optional(),
    expires_in_days: z.number().int().positive().optional(),
})

export const ApiKeyCreatedResponseSchema = ApiKeySchema.extend({
    api_key: z.string(),
})

// User schema
export const UserSchema = z.object({
    uuid: UuidSchema,
    username: z.string(),
    email: z.string().email(),
    first_name: z.string(),
    last_name: z.string(),
    role: z.string(),
    is_active: z.boolean(),
    created_at: TimestampSchema,
    updated_at: TimestampSchema,
})

// Infer TypeScript types from Zod schemas
export type ApiResponse<T> = z.infer<ReturnType<typeof ApiResponseSchema<z.ZodType<T>>>>
export type LoginRequest = z.infer<typeof LoginRequestSchema>
export type LoginResponse = z.infer<typeof LoginResponseSchema>
export type FieldDefinition = z.infer<typeof FieldDefinitionSchema>
export type ClassDefinition = z.infer<typeof ClassDefinitionSchema>
export type DynamicEntity = z.infer<typeof DynamicEntitySchema>
export type ApiKey = z.infer<typeof ApiKeySchema>
export type CreateApiKeyRequest = z.infer<typeof CreateApiKeyRequestSchema>
export type ApiKeyCreatedResponse = z.infer<typeof ApiKeyCreatedResponseSchema>
export type User = z.infer<typeof UserSchema>

// Additional exports for base schemas
export { UuidSchema, TimestampSchema } 