import { z } from 'zod'
import { ApiResponseSchema, PaginatedApiResponseSchema, RoleSchema } from '@/types/schemas'
import type {
    Role,
    CreateRoleRequest,
    UpdateRoleRequest,
    AssignRolesRequest,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class RolesClient extends BaseTypedHttpClient {
    async getRoles(
        page = 1,
        itemsPerPage = 20,
        sortBy?: string | null,
        sortOrder?: 'asc' | 'desc' | null
    ): Promise<{
        data: Role[]
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
            custom?: import('@/types/schemas').RoleCustomData
        }
    }> {
        let url = `/admin/api/v1/roles?page=${page}&per_page=${itemsPerPage}`
        if (sortBy && sortOrder) {
            url += `&sort_by=${sortBy}&sort_order=${sortOrder}`
        }
        const response = await this.paginatedRequest(
            url,
            PaginatedApiResponseSchema(z.array(RoleSchema))
        )
        return response
    }

    async getRole(uuid: string): Promise<Role> {
        return this.request(`/admin/api/v1/roles/${uuid}`, ApiResponseSchema(RoleSchema))
    }

    async createRole(data: CreateRoleRequest): Promise<Role> {
        return this.request('/admin/api/v1/roles', ApiResponseSchema(RoleSchema), {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async updateRole(uuid: string, data: UpdateRoleRequest): Promise<Role> {
        return this.request(`/admin/api/v1/roles/${uuid}`, ApiResponseSchema(RoleSchema), {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async deleteRole(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/roles/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async assignRolesToUser(
        userUuid: string,
        data: AssignRolesRequest
    ): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/roles/users/${userUuid}/roles`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }

    async assignRolesToApiKey(
        apiKeyUuid: string,
        data: AssignRolesRequest
    ): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/roles/api-keys/${apiKeyUuid}/roles`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }
}
