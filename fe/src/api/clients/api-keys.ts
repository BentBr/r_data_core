import type { ApiKeyResponse } from '@/types/generated/ApiKeyResponse'
import type { ApiKeyCreatedResponse } from '@/types/generated/ApiKeyCreatedResponse'
import type { CreateApiKeyRequest, ReassignApiKeyRequest, ResponseMeta } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class ApiKeysClient extends BaseTypedHttpClient {
    async getApiKeys(
        page = 1,
        itemsPerPage = 10,
        sortBy?: string | null,
        sortOrder?: 'asc' | 'desc' | null
    ): Promise<{ data: ApiKeyResponse[]; meta?: ResponseMeta }> {
        let url = `/admin/api/v1/api-keys?page=${page}&per_page=${itemsPerPage}`
        if (sortBy && sortOrder) {
            url += `&sort_by=${sortBy}&sort_order=${sortOrder}`
        }
        return this.paginatedRequest<ApiKeyResponse[]>(url)
    }

    async createApiKey(data: CreateApiKeyRequest): Promise<ApiKeyCreatedResponse> {
        return this.request<ApiKeyCreatedResponse>('/admin/api/v1/api-keys', {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async revokeApiKey(uuid: string): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/api-keys/${uuid}`, {
            method: 'DELETE',
        })
    }

    async reassignApiKey(uuid: string, data: ReassignApiKeyRequest): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/api-keys/${uuid}/reassign`, {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }
}
