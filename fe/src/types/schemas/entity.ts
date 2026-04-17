import { z } from 'zod'
import { UuidSchema, TimestampSchema } from './base'

// Field constraints - API returns nested structure: { type: string, constraints: { ... } }
// We use a permissive type to handle various constraint shapes
export const FieldConstraintsSchema = z
    .object({
        type: z.string().optional(),
        constraints: z.record(z.string(), z.unknown()).optional(),
    })
    .loose()

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
        'Json',
        'ManyToOne',
        'ManyToMany',
        'Select',
        'MultiSelect',
        'Image',
        'File',
        'Password',
    ]),
    description: z.string().nullish(),
    required: z.boolean(),
    indexed: z.boolean(),
    filterable: z.boolean(),
    unique: z.boolean().nullish(),
    default_value: z.unknown().nullish(),
    constraints: FieldConstraintsSchema.nullish(),
    ui_settings: z.record(z.string(), z.unknown()).nullish(),
})

// Entity Definition schema — kept as Zod for form validation
// (CreateEntityDefinitionRequest, UpdateEntityDefinitionRequest use .pick())
export const EntityDefinitionSchema = z.object({
    uuid: UuidSchema.optional(),
    entity_type: z.string(),
    display_name: z.string(),
    description: z
        .string()
        .nullable()
        .transform(v => v ?? undefined)
        .optional(),
    group_name: z
        .string()
        .nullable()
        .transform(v => v ?? undefined)
        .optional(),
    allow_children: z.boolean(),
    icon: z
        .string()
        .nullable()
        .transform(v => v ?? undefined)
        .optional(),
    fields: z.array(FieldDefinitionSchema),
    published: z
        .boolean()
        .nullable()
        .transform(v => v ?? undefined)
        .optional(),
    created_at: TimestampSchema.nullable()
        .transform(v => v ?? undefined)
        .optional(),
    updated_at: TimestampSchema.nullable()
        .transform(v => v ?? undefined)
        .optional(),
    created_by: UuidSchema.nullable()
        .transform(v => v ?? undefined)
        .optional(),
    updated_by: UuidSchema.nullable()
        .transform(v => v ?? undefined)
        .optional(),
    version: z
        .number()
        .int()
        .positive()
        .nullable()
        .transform(v => v ?? undefined)
        .optional(),
})

// Entity request schemas (form validation)
export const CreateEntityRequestSchema = z.object({
    entity_type: z.string(),
    data: z.record(z.string(), z.unknown()),
    parent_uuid: UuidSchema.optional().nullable(),
})

export const UpdateEntityRequestSchema = z.object({
    data: z.record(z.string(), z.unknown()),
    parent_uuid: UuidSchema.optional().nullable(),
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

// Type exports
export type FieldConstraints = z.infer<typeof FieldConstraintsSchema>
export type FieldDefinition = z.infer<typeof FieldDefinitionSchema>
export type EntityDefinition = z.infer<typeof EntityDefinitionSchema>
export type CreateEntityRequest = z.infer<typeof CreateEntityRequestSchema>
export type UpdateEntityRequest = z.infer<typeof UpdateEntityRequestSchema>
export type CreateEntityDefinitionRequest = z.infer<typeof CreateEntityDefinitionRequestSchema>
export type UpdateEntityDefinitionRequest = z.infer<typeof UpdateEntityDefinitionRequestSchema>

// Dynamic entity schemas — kept as Zod for local schema tests and potential future form validation
export const DynamicEntitySchema = z.object({
    entity_type: z.string(),
    field_data: z.record(z.string(), z.unknown()),
    children_count: z.number().int().nullable().optional(),
})

export const EntityResponseSchema = z.object({
    uuid: UuidSchema,
    entity_type: z.string(),
})

// Type exports for dynamic entities
export type DynamicEntity = z.infer<typeof DynamicEntitySchema>
export type EntityResponse = z.infer<typeof EntityResponseSchema>
