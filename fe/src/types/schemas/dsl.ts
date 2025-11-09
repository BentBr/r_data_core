import { z } from 'zod'
import { ApiResponseSchema } from './base'

export const DslFromCsvSchema = z.object({
    type: z.literal('csv'),
    uri: z.string(),
    mapping: z.record(z.string(), z.string()),
})

export const DslFromJsonSchema = z.object({
    type: z.literal('json'),
    uri: z.string(),
    mapping: z.record(z.string(), z.string()),
})

export const DslEntityFilterSchema = z.object({
    field: z.string(),
    value: z.string(),
})

export const DslFromEntitySchema = z.object({
    type: z.literal('entity'),
    entity_definition: z.string(),
    filter: DslEntityFilterSchema,
    mapping: z.record(z.string(), z.string()),
})

export const DslFromSchema = z.discriminatedUnion('type', [
    DslFromCsvSchema,
    DslFromJsonSchema,
    DslFromEntitySchema,
])

export const DslToCsvSchema = z.object({
    type: z.literal('csv'),
    output: z.enum(['api', 'download']),
    mapping: z.record(z.string(), z.string()),
})
export const DslToJsonSchema = z.object({
    type: z.literal('json'),
    output: z.enum(['api', 'download']),
    mapping: z.record(z.string(), z.string()),
})
export const DslToEntitySchema = z.object({
    type: z.literal('entity'),
    entity_definition: z.string(),
    path: z.string(),
    mode: z.enum(['create', 'update']),
    identify: DslEntityFilterSchema.optional(),
    mapping: z.record(z.string(), z.string()),
})
export const DslToSchema = z.discriminatedUnion('type', [
    DslToCsvSchema,
    DslToJsonSchema,
    DslToEntitySchema,
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
export const DslTransformSchema = z.discriminatedUnion('type', [
    DslTransformNoneSchema,
    DslTransformArithmeticSchema,
    DslTransformConcatSchema,
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
    examples: z.array(z.any()).optional(),
})

export type DslStep = z.infer<typeof DslStepSchema>
export type DslValidateRequest = z.infer<typeof DslValidateRequestSchema>
export type DslValidateResponse = z.infer<typeof DslValidateResponseSchema>
export type DslOptionsResponse = z.infer<typeof DslOptionsResponseSchema>

// Re-export ApiResponseSchema for consumers that need it alongside DSL
export { ApiResponseSchema }


