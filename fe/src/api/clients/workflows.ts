import { z } from 'zod'
import {
    ApiResponseSchema,
    PaginatedApiResponseSchema,
    WorkflowSchema,
    WorkflowRunSchema,
    WorkflowRunLogSchema,
    UuidSchema,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'
import { useAuthStore } from '@/stores/auth'

export class WorkflowsClient extends BaseTypedHttpClient {
    async listWorkflows(): Promise<Array<z.infer<typeof WorkflowSchema>>> {
        return this.request('/admin/api/v1/workflows', ApiResponseSchema(z.array(WorkflowSchema)))
    }

    async getWorkflows(
        page = 1,
        itemsPerPage = 20
    ): Promise<{
        data: Array<{
            uuid: string
            name: string
            kind: 'consumer' | 'provider'
            enabled: boolean
            schedule_cron?: string | null
        }>
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
        }
    }> {
        const Schema = z.array(
            z.object({
                uuid: UuidSchema,
                name: z.string(),
                kind: z.enum(['consumer', 'provider']),
                enabled: z.boolean(),
                schedule_cron: z.string().nullable().optional(),
                has_api_endpoint: z.boolean().optional().default(false),
            })
        )
        return this.paginatedRequest(
            `/admin/api/v1/workflows?page=${page}&per_page=${itemsPerPage}`,
            PaginatedApiResponseSchema(Schema)
        )
    }

    async getWorkflow(uuid: string): Promise<z.infer<typeof WorkflowSchema>> {
        return this.request(`/admin/api/v1/workflows/${uuid}`, ApiResponseSchema(WorkflowSchema))
    }

    async createWorkflow(data: {
        name: string
        description?: string | null
        kind: 'consumer' | 'provider'
        enabled: boolean
        schedule_cron?: string | null
        config: unknown
        versioning_disabled?: boolean
    }): Promise<{ uuid: string }> {
        const Schema = z.object({ uuid: UuidSchema })
        return this.request('/admin/api/v1/workflows', ApiResponseSchema(Schema), {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async updateWorkflow(
        uuid: string,
        data: {
            name: string
            description?: string | null
            kind: 'consumer' | 'provider'
            enabled: boolean
            schedule_cron?: string | null
            config: unknown
            versioning_disabled?: boolean
        }
    ): Promise<{ message: string }> {
        const Schema = z.object({ message: z.string() })
        return this.request(`/admin/api/v1/workflows/${uuid}`, ApiResponseSchema(Schema), {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async deleteWorkflow(uuid: string): Promise<{ message: string }> {
        const Schema = z.object({ message: z.string() })
        return this.request(`/admin/api/v1/workflows/${uuid}`, ApiResponseSchema(Schema), {
            method: 'DELETE',
        })
    }

    async runWorkflow(uuid: string): Promise<{ message: string }> {
        const Schema = z.object({ message: z.string() })
        return this.request(`/admin/api/v1/workflows/${uuid}/run`, ApiResponseSchema(Schema), {
            method: 'POST',
        })
    }

    async previewCron(expr: string): Promise<string[]> {
        const Schema = z.array(z.string())
        return this.request(
            `/admin/api/v1/workflows/cron/preview?expr=${encodeURIComponent(expr)}`,
            ApiResponseSchema(Schema)
        )
    }

    async getWorkflowRuns(
        workflowUuid: string,
        page = 1,
        perPage = 20
    ): Promise<{
        data: Array<z.infer<typeof WorkflowRunSchema>>
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
        }
    }> {
        return this.paginatedRequest(
            `/admin/api/v1/workflows/${workflowUuid}/runs?page=${page}&per_page=${perPage}`,
            PaginatedApiResponseSchema(z.array(WorkflowRunSchema))
        )
    }

    async getWorkflowRunLogs(
        runUuid: string,
        page = 1,
        perPage = 50
    ): Promise<{
        data: Array<z.infer<typeof WorkflowRunLogSchema>>
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
        }
    }> {
        return this.paginatedRequest(
            `/admin/api/v1/workflows/runs/${runUuid}/logs?page=${page}&per_page=${perPage}`,
            PaginatedApiResponseSchema(z.array(WorkflowRunLogSchema))
        )
    }

    async getAllWorkflowRuns(
        page = 1,
        perPage = 20
    ): Promise<{
        data: Array<z.infer<typeof WorkflowRunSchema>>
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
        }
    }> {
        return this.paginatedRequest(
            `/admin/api/v1/workflows/runs?page=${page}&per_page=${perPage}`,
            PaginatedApiResponseSchema(z.array(WorkflowRunSchema))
        )
    }

    async uploadRunFile(
        workflowUuid: string,
        file: File
    ): Promise<{ run_uuid: string; staged_items: number }> {
        const form = new FormData()
        form.append('file', file)
        const schema = z.object({
            run_uuid: UuidSchema,
            staged_items: z.number(),
        })
        // Bypass JSON content-type; handle raw fetch here due to multipart
        const authStore = useAuthStore()
        const res = await fetch(
            `${this.baseURL}/admin/api/v1/workflows/${workflowUuid}/run/upload`,
            {
                method: 'POST',
                headers: {
                    ...(authStore.token && { Authorization: `Bearer ${authStore.token}` }),
                },
                body: form,
            }
        )
        if (!res.ok) {
            // Try to extract standardized error
            try {
                const err = await res.json()
                if (err?.message) {
                    throw new Error(err.message)
                }
            } catch {
                // fallthrough
            }
            throw new Error(`HTTP ${res.status}: ${res.statusText}`)
        }
        const json = await res.json()
        const { ApiResponseSchema } = await import('@/types/schemas')
        const parsed = ApiResponseSchema(schema).parse(json)

        return parsed.data
    }

    // DSL endpoints (delegated)
    async getDslFromOptions() {
        const { getDslFromOptions } = await import('./dsl')
        // Pass this instance which has the request method from BaseTypedHttpClient
        return getDslFromOptions(this)
    }
    async getDslToOptions() {
        const { getDslToOptions } = await import('./dsl')
        return getDslToOptions(this)
    }
    async getDslTransformOptions() {
        const { getDslTransformOptions } = await import('./dsl')
        return getDslTransformOptions(this)
    }
    async validateDsl(steps: unknown[]) {
        const { validateDsl } = await import('./dsl')
        return validateDsl(this, steps)
    }

    async listWorkflowVersions(uuid: string): Promise<Array<{
        version_number: number
        created_at: string
        created_by?: string | null
        created_by_name?: string | null
    }>> {
        return this.request(
            `/admin/api/v1/workflows/${uuid}/versions`,
            ApiResponseSchema(
                z.array(
                    z.object({
                        version_number: z.number(),
                        created_at: z.string(),
                        created_by: UuidSchema.nullable().optional(),
                        created_by_name: z.string().nullable().optional(),
                    })
                )
            )
        )
    }

    async getWorkflowVersion(uuid: string, versionNumber: number): Promise<{
        version_number: number
        created_at: string
        created_by?: string | null
        data: Record<string, unknown>
    }> {
        return this.request(
            `/admin/api/v1/workflows/${uuid}/versions/${versionNumber}`,
            ApiResponseSchema(
                z.object({
                    version_number: z.number(),
                    created_at: z.string(),
                    created_by: UuidSchema.nullable().optional(),
                    data: z.any(),
                })
            )
        )
    }
}
