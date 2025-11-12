import { z } from 'zod'
import { UuidSchema, NullableTimestampSchema } from './base'

// Workflow schemas
export const WorkflowSchema = z.object({
    uuid: UuidSchema,
    name: z.string(),
    description: z.string().nullable().optional(),
    kind: z.enum(['consumer', 'provider']),
    enabled: z.boolean(),
    schedule_cron: z.string().nullable().optional(),
    config: z.record(z.string(), z.unknown()),
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
    meta: z.record(z.string(), z.unknown()).optional(),
})

// Type exports
export type Workflow = z.infer<typeof WorkflowSchema>
export type WorkflowRun = z.infer<typeof WorkflowRunSchema>
export type WorkflowRunLog = z.infer<typeof WorkflowRunLogSchema>
