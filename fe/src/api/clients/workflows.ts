import type { WorkflowDetail } from '@/types/generated/WorkflowDetail'
import type { WorkflowSummary } from '@/types/generated/WorkflowSummary'
import type { WorkflowRunLogDto } from '@/types/generated/WorkflowRunLogDto'
import type { DslOptionsResponse, WorkflowRun, WorkflowConfig } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'
import { useAuthStore } from '@/stores/auth'
import { buildApiUrl } from '@/env-check'

export class WorkflowsClient extends BaseTypedHttpClient {
    async listWorkflows(): Promise<Array<WorkflowDetail & { kind: 'consumer' | 'provider' }>> {
        const data = await this.request<WorkflowDetail[]>('/admin/api/v1/workflows')
        return data.map(w => ({ ...w, kind: w.kind.toLowerCase() as 'consumer' | 'provider' }))
    }

    async getWorkflows(
        page = 1,
        itemsPerPage = 20,
        sortBy?: string | null,
        sortOrder?: 'asc' | 'desc' | null
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
        let url = `/admin/api/v1/workflows?page=${page}&per_page=${itemsPerPage}`
        if (sortBy && sortOrder) {
            url += `&sort_by=${sortBy}&sort_order=${sortOrder}`
        }
        const result = await this.paginatedRequest<WorkflowSummary[]>(url)
        return {
            ...result,
            data: result.data.map(w => ({
                ...w,
                kind: w.kind.toLowerCase() as 'consumer' | 'provider',
            })),
        }
    }

    async getWorkflow(uuid: string): Promise<WorkflowDetail & { kind: 'consumer' | 'provider' }> {
        const data = await this.request<WorkflowDetail>(`/admin/api/v1/workflows/${uuid}`)
        return { ...data, kind: data.kind.toLowerCase() as 'consumer' | 'provider' }
    }

    async createWorkflow(data: {
        name: string
        description?: string | null
        kind: 'consumer' | 'provider'
        enabled: boolean
        schedule_cron?: string | null
        config: WorkflowConfig
        versioning_disabled?: boolean
    }): Promise<{ uuid: string }> {
        return this.request<{ uuid: string }>('/admin/api/v1/workflows', {
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
            config: WorkflowConfig
            versioning_disabled?: boolean
        }
    ): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/workflows/${uuid}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async deleteWorkflow(uuid: string): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/workflows/${uuid}`, {
            method: 'DELETE',
        })
    }

    async runWorkflow(uuid: string): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/workflows/${uuid}/run`, {
            method: 'POST',
        })
    }

    async previewCron(expr: string): Promise<string[]> {
        return this.request<string[]>(
            `/admin/api/v1/workflows/cron/preview?expr=${encodeURIComponent(expr)}`
        )
    }

    async getWorkflowRuns(
        workflowUuid: string,
        page = 1,
        perPage = 20
    ): Promise<{
        data: WorkflowRun[]
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
        return this.paginatedRequest<WorkflowRun[]>(
            `/admin/api/v1/workflows/${workflowUuid}/runs?page=${page}&per_page=${perPage}`
        )
    }

    async getWorkflowRunLogs(
        runUuid: string,
        page = 1,
        perPage = 50
    ): Promise<{
        data: WorkflowRunLogDto[]
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
        return this.paginatedRequest<WorkflowRunLogDto[]>(
            `/admin/api/v1/workflows/runs/${runUuid}/logs?page=${page}&per_page=${perPage}`
        )
    }

    async getAllWorkflowRuns(
        page = 1,
        perPage = 20
    ): Promise<{
        data: WorkflowRun[]
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
        return this.paginatedRequest<WorkflowRun[]>(
            `/admin/api/v1/workflows/runs?page=${page}&per_page=${perPage}`
        )
    }

    async uploadRunFile(
        workflowUuid: string,
        file: File
    ): Promise<{ run_uuid: string; staged_items: number }> {
        const form = new FormData()
        form.append('file', file)
        // Bypass JSON content-type; handle raw fetch here due to multipart
        const authStore = useAuthStore()
        const res = await fetch(buildApiUrl(`/admin/api/v1/workflows/${workflowUuid}/run/upload`), {
            method: 'POST',
            headers: {
                ...(authStore.token && { Authorization: `Bearer ${authStore.token}` }),
            },
            body: form,
        })
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
        const json = (await res.json()) as {
            status: string
            message: string
            data?: { run_uuid: string; staged_items: number } | null
        }

        if (!json.data) {
            throw new Error('No data in upload response')
        }
        return json.data
    }

    // DSL endpoints
    async getDslFromOptions(): Promise<DslOptionsResponse> {
        return this.request<DslOptionsResponse>('/admin/api/v1/dsl/from/options')
    }

    async getDslToOptions(): Promise<DslOptionsResponse> {
        return this.request<DslOptionsResponse>('/admin/api/v1/dsl/to/options')
    }

    async getDslTransformOptions(): Promise<DslOptionsResponse> {
        return this.request<DslOptionsResponse>('/admin/api/v1/dsl/transform/options')
    }

    async validateDsl(steps: import('@/types/schemas').DslStep[]): Promise<{ valid: boolean }> {
        return this.request<{ valid: boolean }>('/admin/api/v1/dsl/validate', {
            method: 'POST',
            body: JSON.stringify({ steps }),
        })
    }

    async listWorkflowVersions(uuid: string): Promise<
        Array<{
            version_number: number
            created_at: string
            created_by?: string | null
            created_by_name?: string | null
        }>
    > {
        return this.request<
            Array<{
                version_number: number
                created_at: string
                created_by?: string | null
                created_by_name?: string | null
            }>
        >(`/admin/api/v1/workflows/${uuid}/versions`)
    }

    async getWorkflowVersion(
        uuid: string,
        versionNumber: number
    ): Promise<{
        version_number: number
        created_at: string
        created_by?: string | null
        data: Record<string, unknown>
    }> {
        return this.request<{
            version_number: number
            created_at: string
            created_by?: string | null
            data: Record<string, unknown>
        }>(`/admin/api/v1/workflows/${uuid}/versions/${versionNumber}`)
    }
}
