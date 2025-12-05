import { z } from 'zod'
import {
    ApiResponseSchema,
    PaginatedApiResponseSchema,
    PermissionSchemeSchema,
} from '@/types/schemas'
import type {
    PermissionScheme,
    CreatePermissionSchemeRequest,
    UpdatePermissionSchemeRequest,
    AssignSchemesRequest,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class PermissionsClient extends BaseTypedHttpClient {
    async getPermissionSchemes(
        page = 1,
        itemsPerPage = 20
    ): Promise<{
        data: PermissionScheme[]
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
            custom?: unknown
        }
    }> {
        const response = await this.paginatedRequest(
            `/admin/api/v1/permissions?page=${page}&per_page=${itemsPerPage}`,
            PaginatedApiResponseSchema(z.array(PermissionSchemeSchema))
        )
        return response
    }

    async getPermissionScheme(uuid: string): Promise<PermissionScheme> {
        return this.request(
            `/admin/api/v1/permissions/${uuid}`,
            ApiResponseSchema(PermissionSchemeSchema)
        )
    }

    async createPermissionScheme(data: CreatePermissionSchemeRequest): Promise<PermissionScheme> {
        return this.request(
            '/admin/api/v1/permissions',
            ApiResponseSchema(PermissionSchemeSchema),
            {
                method: 'POST',
                body: JSON.stringify(data),
            }
        )
    }

    async updatePermissionScheme(
        uuid: string,
        data: UpdatePermissionSchemeRequest
    ): Promise<PermissionScheme> {
        return this.request(
            `/admin/api/v1/permissions/${uuid}`,
            ApiResponseSchema(PermissionSchemeSchema),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }

    async deletePermissionScheme(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/permissions/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async assignSchemesToUser(
        userUuid: string,
        data: AssignSchemesRequest
    ): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/permissions/users/${userUuid}/schemes`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }

    async assignSchemesToApiKey(
        apiKeyUuid: string,
        data: AssignSchemesRequest
    ): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/permissions/api-keys/${apiKeyUuid}/schemes`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }
}
