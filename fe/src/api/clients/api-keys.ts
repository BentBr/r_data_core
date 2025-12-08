import { z } from 'zod'
import {
    ApiResponseSchema,
    PaginatedApiResponseSchema,
    ApiKeySchema,
    ApiKeyCreatedResponseSchema,
} from '@/types/schemas'
import type {
    ApiKey,
    CreateApiKeyRequest,
    ApiKeyCreatedResponse,
    ReassignApiKeyRequest,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class ApiKeysClient extends BaseTypedHttpClient {
    async getApiKeys(
        page = 1,
        itemsPerPage = 10,
        sortBy?: string | null,
        sortOrder?: 'asc' | 'desc' | null
    ): Promise<{
        data: ApiKey[]
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
            request_id?: string
            timestamp?: string
            custom?: import('@/types/schemas').ApiKeyCustomData
        }
    }> {
        let url = `/admin/api/v1/api-keys?page=${page}&per_page=${itemsPerPage}`
        if (sortBy && sortOrder) {
            url += `&sort_by=${sortBy}&sort_order=${sortOrder}`
        }
        const response = await this.paginatedRequest(
            url,
            PaginatedApiResponseSchema(z.array(ApiKeySchema))
        )
        return response
    }

    async createApiKey(data: CreateApiKeyRequest): Promise<ApiKeyCreatedResponse> {
        return this.request(
            '/admin/api/v1/api-keys',
            ApiResponseSchema(ApiKeyCreatedResponseSchema),
            {
                method: 'POST',
                body: JSON.stringify(data),
            }
        )
    }

    async revokeApiKey(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/api-keys/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async reassignApiKey(uuid: string, data: ReassignApiKeyRequest): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/api-keys/${uuid}/reassign`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }
}
