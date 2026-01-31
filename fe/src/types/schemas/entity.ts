import { z } from 'zod'
import { UuidSchema, TimestampSchema } from './base'

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
    ]),
    description: z.string().optional(),
    required: z.boolean(),
    indexed: z.boolean(),
    filterable: z.boolean(),
    default_value: z.unknown().nullish(),
    constraints: z.record(z.string(), z.unknown()).nullish(),
    ui_settings: z.record(z.string(), z.unknown()).nullish(),
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
// Backend returns: { entity_type, field_data: { uuid, path, etc., ...customFields }, children_count? }
export const DynamicEntitySchema = z.object({
    entity_type: z.string(),
    field_data: z.record(z.string(), z.unknown()),
    children_count: z.number().int().nullable().optional(),
})

// Entity request/response schemas
export const CreateEntityRequestSchema = z.object({
    entity_type: z.string(),
    data: z.record(z.string(), z.unknown()),
    parent_uuid: UuidSchema.optional().nullable(),
})

export const UpdateEntityRequestSchema = z.object({
    data: z.record(z.string(), z.unknown()),
    parent_uuid: UuidSchema.optional().nullable(),
})

export const EntityResponseSchema = z.object({
    uuid: UuidSchema,
    entity_type: z.string(),
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
export type FieldDefinition = z.infer<typeof FieldDefinitionSchema>
export type EntityDefinition = z.infer<typeof EntityDefinitionSchema>
export type DynamicEntity = z.infer<typeof DynamicEntitySchema>
export type CreateEntityRequest = z.infer<typeof CreateEntityRequestSchema>
export type UpdateEntityRequest = z.infer<typeof UpdateEntityRequestSchema>
export type EntityResponse = z.infer<typeof EntityResponseSchema>
export type CreateEntityDefinitionRequest = z.infer<typeof CreateEntityDefinitionRequestSchema>
export type UpdateEntityDefinitionRequest = z.infer<typeof UpdateEntityDefinitionRequestSchema>
