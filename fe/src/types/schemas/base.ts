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
export const NullableUuidSchema = z.preprocess(
    val => {
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

// Validation error schemas — kept as Zod because they are used for runtime parsing
// (.parse()) in api/clients/base.ts and api/http-client.ts.
// Generated equivalents: ValidationViolation, ValidationErrorResponse in generated/
export const ValidationViolationSchema = z.object({
    field: z.string(),
    message: z.string(),
    code: z.string().optional(),
})

export const ValidationErrorResponseSchema = z.object({
    message: z.string(),
    violations: z.array(ValidationViolationSchema),
})

// Pagination and meta types (plain TypeScript — no Zod parsing needed)
export interface Pagination {
    total: number
    page: number
    per_page: number
    total_pages: number
    has_previous: boolean
    has_next: boolean
}

export interface Meta {
    pagination?: Pagination
    request_id?: string
    timestamp?: string
    custom?: unknown
}

// Type re-exports from generated — for consumers that only need the type, not runtime parsing
export type { ValidationViolation } from '../generated/ValidationViolation'
export type { ValidationErrorResponse } from '../generated/ValidationErrorResponse'
export type { PaginationMeta } from '../generated/PaginationMeta'
export type { ResponseMeta } from '../generated/ResponseMeta'
