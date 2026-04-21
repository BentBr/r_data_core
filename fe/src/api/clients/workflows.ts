import type { WorkflowDetail } from '@/types/generated/WorkflowDetail'
import type { WorkflowSummary } from '@/types/generated/WorkflowSummary'
import type { WorkflowRunLogDto } from '@/types/generated/WorkflowRunLogDto'
import type { CreateWorkflowResponse } from '@/types/generated/CreateWorkflowResponse'
import type { CreateWorkflowRequest } from '@/types/generated/CreateWorkflowRequest'
import type { UpdateWorkflowRequest } from '@/types/generated/UpdateWorkflowRequest'
import type { WorkflowVersionMeta } from '@/types/generated/WorkflowVersionMeta'
import type { WorkflowVersionPayload } from '@/types/generated/WorkflowVersionPayload'
import type { WorkflowRunUploadResponse } from '@/types/generated/WorkflowRunUploadResponse'
import type { DslValidateResponse } from '@/types/generated/DslValidateResponse'
import type { PaginationQuery } from '@/types/generated/PaginationQuery'
import type { SortingQuery } from '@/types/generated/SortingQuery'
import type { DslOptionsResponse, WorkflowRun, ResponseMeta } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'
import { buildListQueryString } from './query'
import { useAuthStore } from '@/stores/auth'
import { buildApiUrl } from '@/env-check'

export class WorkflowsClient extends BaseTypedHttpClient {
    async listWorkflows(): Promise<WorkflowSummary[]> {
        return this.request<WorkflowSummary[]>('/admin/api/v1/workflows')
    }

    async getWorkflows(
        pagination: PaginationQuery,
        sorting?: SortingQuery | null
    ): Promise<{ data: WorkflowSummary[]; meta?: ResponseMeta }> {
        return this.paginatedRequest<WorkflowSummary[]>(
            `/admin/api/v1/workflows${buildListQueryString(pagination, sorting)}`
        )
    }

    async getWorkflow(uuid: string): Promise<WorkflowDetail> {
        return this.request<WorkflowDetail>(`/admin/api/v1/workflows/${uuid}`)
    }

    async createWorkflow(data: CreateWorkflowRequest): Promise<CreateWorkflowResponse> {
        return this.request<CreateWorkflowResponse>('/admin/api/v1/workflows', {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async updateWorkflow(uuid: string, data: UpdateWorkflowRequest): Promise<{ message: string }> {
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
        pagination: PaginationQuery
    ): Promise<{ data: WorkflowRun[]; meta?: ResponseMeta }> {
        return this.paginatedRequest<WorkflowRun[]>(
            `/admin/api/v1/workflows/${workflowUuid}/runs${buildListQueryString(pagination)}`
        )
    }

    async getWorkflowRunLogs(
        runUuid: string,
        pagination: PaginationQuery
    ): Promise<{ data: WorkflowRunLogDto[]; meta?: ResponseMeta }> {
        return this.paginatedRequest<WorkflowRunLogDto[]>(
            `/admin/api/v1/workflows/runs/${runUuid}/logs${buildListQueryString(pagination)}`
        )
    }

    async getAllWorkflowRuns(
        pagination: PaginationQuery
    ): Promise<{ data: WorkflowRun[]; meta?: ResponseMeta }> {
        return this.paginatedRequest<WorkflowRun[]>(
            `/admin/api/v1/workflows/runs${buildListQueryString(pagination)}`
        )
    }

    async uploadRunFile(workflowUuid: string, file: File): Promise<WorkflowRunUploadResponse> {
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
            data?: WorkflowRunUploadResponse | null
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

    async validateDsl(steps: import('@/types/schemas').DslStep[]): Promise<DslValidateResponse> {
        return this.request<DslValidateResponse>('/admin/api/v1/dsl/validate', {
            method: 'POST',
            body: JSON.stringify({ steps }),
        })
    }

    async listWorkflowVersions(uuid: string): Promise<WorkflowVersionMeta[]> {
        return this.request<WorkflowVersionMeta[]>(`/admin/api/v1/workflows/${uuid}/versions`)
    }

    async getWorkflowVersion(uuid: string, versionNumber: number): Promise<WorkflowVersionPayload> {
        return this.request<WorkflowVersionPayload>(
            `/admin/api/v1/workflows/${uuid}/versions/${versionNumber}`
        )
    }
}
