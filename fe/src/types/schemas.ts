import { z } from 'zod'

// Base schemas for common patterns - handle both traditional UUID and UUIDv7 formats
const UuidSchema = z.string().refine(
    val => {
        // UUID format: 8-4-4-4-12 hexadecimal (covers UUID v1-v7)
        const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
        return uuidRegex.test(val)
    },
    {
        message: 'Invalid UUID format',
    }
)

// More flexible timestamp schema to handle backend nanosecond precision
const TimestampSchema = z.string().refine(
    val => {
        // Allow ISO 8601 format with varying precision
        const isoRegex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z?$/
        return isoRegex.test(val) && !isNaN(Date.parse(val))
    },
    {
        message: 'Invalid timestamp format, expected ISO 8601',
    }
)

// Nullable timestamp schema for fields that can be null
const NullableTimestampSchema = z
    .string()
    .refine(
        val => {
            // Allow ISO 8601 format with varying precision
            const isoRegex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z?$/
            return isoRegex.test(val) && !isNaN(Date.parse(val))
        },
        {
            message: 'Invalid timestamp format, expected ISO 8601',
        }
    )
    .nullable()

// Validation error schemas (Symfony-style)
export const ValidationViolationSchema = z.object({
    field: z.string(),
    message: z.string(),
    code: z.string().optional(),
})

export const ValidationErrorResponseSchema = z.object({
    message: z.string(),
    violations: z.array(ValidationViolationSchema),
})

// Common schemas for reusable components
export const PaginationSchema = z.object({
    total: z.number(),
    page: z.number(),
    per_page: z.number(),
    total_pages: z.number(),
    has_previous: z.boolean(),
    has_next: z.boolean(),
})

export const MetaSchema = z.object({
    pagination: PaginationSchema.optional(),
    request_id: UuidSchema.optional(),
    timestamp: TimestampSchema.optional(),
    custom: z.unknown().optional(),
})

// Common table/action schemas
export const TableActionSchema = z.object({
    icon: z.string(),
    color: z.string().optional(),
    tooltip: z.string().optional(),
    disabled: z.boolean().optional(),
    loading: z.boolean().optional(),
})

export const TableColumnSchema = z.object({
    key: z.string(),
    title: z.string(),
    sortable: z.boolean().optional(),
    align: z.enum(['start', 'center', 'end']).optional(),
    width: z.string().optional(),
    fixed: z.boolean().optional(),
})

// Tree view schemas
export const TreeNodeSchema: z.ZodType<{
    id: string
    title: string
    icon?: string
    color?: string
    children?: Array<{
        id: string
        title: string
        icon?: string
        color?: string
        children?: Array<unknown>
        expanded?: boolean
        selected?: boolean
        disabled?: boolean
        entity_type?: string
        uuid?: string
        display_name?: string
        published?: boolean
    }>
    expanded?: boolean
    selected?: boolean
    disabled?: boolean
    entity_type?: string
    uuid?: string
    display_name?: string
    published?: boolean
}> = z.object({
    id: z.string(),
    title: z.string(),
    icon: z.string().optional(),
    color: z.string().optional(),
    children: z.array(z.lazy(() => TreeNodeSchema)).optional(),
    expanded: z.boolean().optional(),
    selected: z.boolean().optional(),
    disabled: z.boolean().optional(),
    // Allow additional properties for specific use cases
    entity_type: z.string().optional(),
    uuid: z.string().optional(),
    display_name: z.string().optional(),
    published: z.boolean().optional(),
    hasChildren: z.boolean().optional(),
    path: z.string().optional(),
})

// Snackbar schemas
export const SnackbarConfigSchema = z.object({
    message: z.string(),
    color: z.enum(['success', 'error', 'warning', 'info']).optional(),
    timeout: z.number().optional(),
    persistent: z.boolean().optional(),
})

// Dialog schemas
export const DialogConfigSchema = z.object({
    title: z.string(),
    width: z.string().optional(),
    persistent: z.boolean().optional(),
    maxWidth: z.string().optional(),
})

// Form field schemas
export const FormFieldSchema = z.object({
    name: z.string(),
    label: z.string(),
    type: z.enum(['text', 'textarea', 'select', 'switch', 'number', 'date', 'email', 'password']),
    required: z.boolean().optional(),
    rules: z.array(z.string()).optional(),
    options: z.array(z.object({ value: z.string(), label: z.string() })).optional(),
    placeholder: z.string().optional(),
    hint: z.string().optional(),
    disabled: z.boolean().optional(),
})

// Generic API Response wrapper schema
export const ApiResponseSchema = <T extends z.ZodTypeAny>(dataSchema: T) =>
    z.object({
        status: z.enum(['Success', 'Error']),
        message: z.string(),
        data: dataSchema.optional().nullable(),
        meta: MetaSchema.nullish(), // Allow null, undefined, or the object
    })

// Paginated API Response wrapper schema
export const PaginatedApiResponseSchema = <T extends z.ZodTypeAny>(dataSchema: T) =>
    z.object({
        status: z.enum(['Success', 'Error']),
        message: z.string(),
        data: dataSchema,
        meta: MetaSchema,
    })

// Auth schemas
export const LoginRequestSchema = z.object({
    username: z.string().min(3),
    password: z.string().min(8),
})

export const LoginResponseSchema = z.object({
    access_token: z.string(),
    refresh_token: z.string(),
    user_uuid: UuidSchema,
    username: z.string(),
    role: z.string(),
    access_expires_at: TimestampSchema,
    refresh_expires_at: TimestampSchema,
})

// Field Definition schema
export const FieldDefinitionSchema = z.object({
    name: z.string(),
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
        'Uuid',
        'ManyToOne',
        'ManyToMany',
        'Select',
        'MultiSelect',
        'Image',
        'File',
    ]),
    description: z.string().optional(),
    required: z.boolean(),
    indexed: z.boolean(),
    filterable: z.boolean(),
    default_value: z.any().optional(),
    constraints: z.any().optional(),
    ui_settings: z.any().optional(),
})

// Entity Definition schema
export const EntityDefinitionSchema = z.object({
    uuid: UuidSchema.optional(),
    entity_type: z.string(),
    display_name: z.string(),
    description: z.string().optional(),
    group_name: z.string().optional(),
    allow_children: z.boolean(),
    icon: z.string().optional(),
    fields: z.array(FieldDefinitionSchema),
    published: z.boolean().optional(),
    created_at: TimestampSchema.optional(),
    updated_at: TimestampSchema.optional(),
    created_by: UuidSchema.optional(),
    updated_by: UuidSchema.optional(),
    version: z.number().int().positive().optional(),
})

// Dynamic Entity schema
// Backend returns: { entity_type, field_data: { uuid, path, etc., ...customFields } }
export const DynamicEntitySchema = z.object({
    entity_type: z.string(),
    field_data: z.record(z.string(), z.any()),
})

// Entity request/response schemas
export const CreateEntityRequestSchema = z.object({
    entity_type: z.string(),
    data: z.record(z.string(), z.any()),
    parent_uuid: UuidSchema.optional().nullable(),
})

export const UpdateEntityRequestSchema = z.object({
    data: z.record(z.string(), z.any()),
    parent_uuid: UuidSchema.optional().nullable(),
})

export const EntityResponseSchema = z.object({
    uuid: UuidSchema,
    entity_type: z.string(),
})

// API Key schema
export const ApiKeySchema = z.object({
    uuid: UuidSchema,
    name: z.string(),
    description: z.string().optional(),
    is_active: z.boolean(),
    created_at: TimestampSchema,
    expires_at: NullableTimestampSchema,
    last_used_at: NullableTimestampSchema,
    created_by: UuidSchema,
    user_uuid: UuidSchema,
    published: z.boolean(),
})

export const CreateApiKeyRequestSchema = z.object({
    name: z.string().min(1),
    description: z.string().optional(),
    expires_in_days: z.number().int().positive().optional(),
})

export const CreateEntityDefinitionRequestSchema = EntityDefinitionSchema.pick({
    entity_type: true,
    display_name: true,
    description: true,
    group_name: true,
    allow_children: true,
    icon: true,
    fields: true,
    published: true,
})

export const UpdateEntityDefinitionRequestSchema = EntityDefinitionSchema.pick({
    entity_type: true,
    display_name: true,
    description: true,
    group_name: true,
    allow_children: true,
    icon: true,
    fields: true,
    published: true,
})

export const ApiKeyCreatedResponseSchema = ApiKeySchema.extend({
    api_key: z.string(),
})

export const ReassignApiKeyRequestSchema = z.object({
    user_uuid: UuidSchema,
})

export const ReassignApiKeyResponseSchema = z.object({
    message: z.string(),
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
    is_admin: z.boolean(),
    created_at: TimestampSchema,
    updated_at: TimestampSchema,
})

// Infer TypeScript types from Zod schemas
export type ApiResponse<T> = z.infer<ReturnType<typeof ApiResponseSchema<z.ZodType<T>>>>
export type LoginRequest = z.infer<typeof LoginRequestSchema>
export type LoginResponse = z.infer<typeof LoginResponseSchema>

// Refresh token schemas
export const RefreshTokenRequestSchema = z.object({
    refresh_token: z.string(),
})

export const RefreshTokenResponseSchema = z.object({
    access_token: z.string(),
    refresh_token: z.string(),
    access_expires_at: TimestampSchema,
    refresh_expires_at: TimestampSchema,
})
export const LogoutRequestSchema = z.object({
    refresh_token: z.string(),
})

export type RefreshTokenRequest = z.infer<typeof RefreshTokenRequestSchema>
export type RefreshTokenResponse = z.infer<typeof RefreshTokenResponseSchema>
export type LogoutRequest = z.infer<typeof LogoutRequestSchema>
export type ReassignApiKeyRequest = z.infer<typeof ReassignApiKeyRequestSchema>
export type ReassignApiKeyResponse = z.infer<typeof ReassignApiKeyResponseSchema>
export type FieldDefinition = z.infer<typeof FieldDefinitionSchema>
export type EntityDefinition = z.infer<typeof EntityDefinitionSchema>
export type DynamicEntity = z.infer<typeof DynamicEntitySchema>
export type CreateEntityRequest = z.infer<typeof CreateEntityRequestSchema>
export type UpdateEntityRequest = z.infer<typeof UpdateEntityRequestSchema>
export type EntityResponse = z.infer<typeof EntityResponseSchema>
export type ApiKey = z.infer<typeof ApiKeySchema>
export type CreateApiKeyRequest = z.infer<typeof CreateApiKeyRequestSchema>
export type ApiKeyCreatedResponse = z.infer<typeof ApiKeyCreatedResponseSchema>
export type CreateEntityDefinitionRequest = z.infer<typeof CreateEntityDefinitionRequestSchema>
export type UpdateEntityDefinitionRequest = z.infer<typeof UpdateEntityDefinitionRequestSchema>
export type User = z.infer<typeof UserSchema>

// Common type exports
export type Pagination = z.infer<typeof PaginationSchema>
export type Meta = z.infer<typeof MetaSchema>
export type TableAction = z.infer<typeof TableActionSchema>
export type TableColumn = z.infer<typeof TableColumnSchema>
export type TreeNode = z.infer<typeof TreeNodeSchema>
export type SnackbarConfig = z.infer<typeof SnackbarConfigSchema>
export type DialogConfig = z.infer<typeof DialogConfigSchema>
export type FormField = z.infer<typeof FormFieldSchema>

// Additional exports for base schemas
export { UuidSchema, TimestampSchema, NullableTimestampSchema }
