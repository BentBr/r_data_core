import type { RoleResponse } from '@/types/generated/RoleResponse'
import type { CreateRoleRequest, UpdateRoleRequest, AssignRolesRequest } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class RolesClient extends BaseTypedHttpClient {
    async getRoles(
        page = 1,
        itemsPerPage = 20
    ): Promise<{
        data: RoleResponse[]
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
        return this.paginatedRequest<RoleResponse[]>(
            `/admin/api/v1/roles?page=${page}&per_page=${itemsPerPage}`
        )
    }

    async getRole(uuid: string): Promise<RoleResponse> {
        return this.request<RoleResponse>(`/admin/api/v1/roles/${uuid}`)
    }

    async createRole(data: CreateRoleRequest): Promise<RoleResponse> {
        return this.request<RoleResponse>('/admin/api/v1/roles', {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async updateRole(uuid: string, data: UpdateRoleRequest): Promise<RoleResponse> {
        return this.request<RoleResponse>(`/admin/api/v1/roles/${uuid}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async deleteRole(uuid: string): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/roles/${uuid}`, {
            method: 'DELETE',
        })
    }

    async assignRolesToUser(
        userUuid: string,
        data: AssignRolesRequest
    ): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/roles/users/${userUuid}/roles`, {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async assignRolesToApiKey(
        apiKeyUuid: string,
        data: AssignRolesRequest
    ): Promise<{ message: string }> {
        return this.request<{ message: string }>(
            `/admin/api/v1/roles/api-keys/${apiKeyUuid}/roles`,
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }
}
