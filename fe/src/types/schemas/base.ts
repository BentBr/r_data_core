import { z } from 'zod'

// Base schemas for common patterns - UUID v7 only
export const UuidSchema = z.string().refine(
    val => {
        const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
        return uuidRegex.test(val)
    },
    { message: 'Invalid UUID (must be v7)' }
)

// Nullable UUID schema that transforms nil UUIDs to null
// Handles cases where backend returns Uuid::nil() (00000000-0000-0000-0000-000000000000) for NULL database values
export const NullableUuidSchema = z.preprocess(
    (val) => {
        // Transform nil UUID (00000000-0000-0000-0000-000000000000) to null
        if (val === '00000000-0000-0000-0000-000000000000' || val === null || val === undefined) {
            return null
        }
        return val
    },
    z.union([UuidSchema, z.null()])
)

// Timestamps
export const TimestampSchema = z.string().refine(
    val => {
        const isoRegex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z?$/
        return isoRegex.test(val) && !isNaN(Date.parse(val))
    },
    { message: 'Invalid timestamp format, expected ISO 8601' }
)

export const NullableTimestampSchema = z
    .string()
    .refine(
        val => {
            const isoRegex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z?$/
            return isoRegex.test(val) && !isNaN(Date.parse(val))
        },
        { message: 'Invalid timestamp format, expected ISO 8601' }
    )
    .nullable()

// Validation
export const ValidationViolationSchema = z.object({
    field: z.string(),
    message: z.string(),
    code: z.string().optional(),
})

export const ValidationErrorResponseSchema = z.object({
    message: z.string(),
    violations: z.array(ValidationViolationSchema),
})

// Meta/pagination
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

// Generic API Response wrappers
export const ApiResponseSchema = <T extends z.ZodTypeAny>(dataSchema: T) =>
    z.object({
        status: z.enum(['Success', 'Error']),
        message: z.string(),
        data: dataSchema.optional().nullable(),
        meta: MetaSchema.nullish(),
    })

export const PaginatedApiResponseSchema = <T extends z.ZodTypeAny>(dataSchema: T) =>
    z.object({
        status: z.enum(['Success', 'Error']),
        message: z.string(),
        data: dataSchema,
        meta: MetaSchema,
    })
