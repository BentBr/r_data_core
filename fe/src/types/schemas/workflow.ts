import { z } from 'zod'
import { UuidSchema, NullableTimestampSchema } from './base'

// Workflow schemas
export const WorkflowSchema = z.object({
    uuid: UuidSchema,
    name: z.string(),
    description: z.string().nullable().optional(),
    kind: z
        .string()
        .transform(val => val.toLowerCase())
        .pipe(z.enum(['consumer', 'provider'])),
    enabled: z.boolean(),
    schedule_cron: z.string().nullable().optional(),
    config: z.record(z.string(), z.unknown()),
    versioning_disabled: z.boolean().optional().default(false),
})

export const WorkflowRunSchema = z.object({
    uuid: UuidSchema,
    status: z.string(),
    queued_at: NullableTimestampSchema,
    finished_at: NullableTimestampSchema,
    processed_items: z.number().nullable().optional(),
    failed_items: z.number().nullable().optional(),
})

export const WorkflowRunLogSchema = z.object({
    uuid: UuidSchema,
    ts: z.string(),
    level: z.string(),
    message: z.string(),
    meta: z.record(z.string(), z.unknown()).nullish(), // Allow null or undefined
})

// Type exports
export type Workflow = z.infer<typeof WorkflowSchema>
export type WorkflowRun = z.infer<typeof WorkflowRunSchema>
export type WorkflowRunLog = z.infer<typeof WorkflowRunLogSchema>

/**
 * Workflow configuration object
 * This is a flexible type that can contain any workflow-specific configuration
 * The actual structure depends on the workflow kind and DSL configuration
 */
export interface WorkflowConfig extends Record<string, unknown> {
    steps?: unknown[]
    from?: unknown
    to?: unknown
    transform?: unknown
    auth?: unknown
    csv_options?: unknown

    [key: string]: unknown
}
