// Type exports — use generated types where possible, fallback interfaces for bigint/number compat

// Workflow uses generated WorkflowDetail (kind: string — clients apply lowercase)
export type { WorkflowDetail as Workflow } from '../generated/WorkflowDetail'

// WorkflowRun: generated WorkflowRunSummary uses bigint for processed_items/failed_items
// (Rust u64 → TS bigint), but JSON transport sends numbers. Use compatible interface.
export interface WorkflowRun {
    uuid: string
    status: string
    queued_at: string | null
    started_at?: string | null
    finished_at: string | null
    processed_items?: number | null
    failed_items?: number | null
}

// WorkflowRunLog uses generated WorkflowRunLogDto
export type { WorkflowRunLogDto as WorkflowRunLog } from '../generated/WorkflowRunLogDto'

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

/**
 * Opt-in throttling for a workflow's public endpoint.
 *
 * The backend treats a missing block, `enabled: false`, or a zero in either
 * number as "no limit at all" (`WorkflowRateLimit::from_config`), so the form
 * must not let a zero through while showing the switch as on.
 */
export interface WorkflowRateLimit {
    enabled: boolean
    max_requests: number
    window_minutes: number
}

export const DEFAULT_RATE_LIMIT: WorkflowRateLimit = {
    enabled: false,
    max_requests: 10,
    window_minutes: 60,
}
