// Workflow uses generated WorkflowDetail (kind: string — clients apply lowercase)
export type { WorkflowDetail as Workflow } from '../generated/WorkflowDetail'

// WorkflowRun aliases the generated WorkflowRunSummary directly.
export type { WorkflowRunSummary as WorkflowRun } from '../generated/WorkflowRunSummary'

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
