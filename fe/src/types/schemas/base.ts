import { z } from 'zod'
import type { ValidationViolation } from '../generated/ValidationViolation'
import type { ValidationErrorResponse } from '../generated/ValidationErrorResponse'

// Base schemas for common patterns - UUID v7 only
export const UuidSchema = z.string().refine(
    val => {
        const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
        return uuidRegex.test(val)
    },
    { message: 'Invalid UUID (must be v7)' }
)

// Timestamps
export const TimestampSchema = z.string().refine(
    val => {
        const isoRegex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z?$/
        return isoRegex.test(val) && !isNaN(Date.parse(val))
    },
    { message: 'Invalid timestamp format, expected ISO 8601' }
)

// Zod runtime parsers bound to generated BE types via `satisfies` — the FE cannot
// diverge from the backend contract because a shape mismatch fails compilation.
export const ValidationViolationSchema = z.object({
    field: z.string(),
    message: z.string(),
    code: z.string().nullable(),
}) satisfies z.ZodType<ValidationViolation>

export const ValidationErrorResponseSchema = z.object({
    message: z.string(),
    violations: z.array(ValidationViolationSchema),
}) satisfies z.ZodType<ValidationErrorResponse>

// Type re-exports — response-envelope shapes come straight from the generated BE bindings.
export type { ValidationViolation } from '../generated/ValidationViolation'
export type { ValidationErrorResponse } from '../generated/ValidationErrorResponse'
export type { PaginationMeta } from '../generated/PaginationMeta'
export type { ResponseMeta } from '../generated/ResponseMeta'
