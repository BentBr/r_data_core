import type { ApiKeyResponse } from '@/types/generated/ApiKeyResponse'
import type { ApiKeyCreatedResponse } from '@/types/generated/ApiKeyCreatedResponse'
import type { PaginationQuery } from '@/types/generated/PaginationQuery'
import type { SortingQuery } from '@/types/generated/SortingQuery'
import type { CreateApiKeyRequest, ReassignApiKeyRequest, ResponseMeta } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'
import { buildListQueryString } from './query'

export class ApiKeysClient extends BaseTypedHttpClient {
    async getApiKeys(
        pagination: PaginationQuery,
        sorting?: SortingQuery | null
    ): Promise<{ data: ApiKeyResponse[]; meta?: ResponseMeta }> {
        return this.paginatedRequest<ApiKeyResponse[]>(
            `/admin/api/v1/api-keys${buildListQueryString(pagination, sorting)}`
        )
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
