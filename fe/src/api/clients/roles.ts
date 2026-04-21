import type { RoleResponse } from '@/types/generated/RoleResponse'
import type { PaginationQuery } from '@/types/generated/PaginationQuery'
import type { SortingQuery } from '@/types/generated/SortingQuery'
import type {
    CreateRoleRequest,
    UpdateRoleRequest,
    AssignRolesRequest,
    ResponseMeta,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'
import { buildListQueryString } from './query'

export class RolesClient extends BaseTypedHttpClient {
    async getRoles(
        pagination: PaginationQuery,
        sorting?: SortingQuery | null
    ): Promise<{ data: RoleResponse[]; meta?: ResponseMeta }> {
        return this.paginatedRequest<RoleResponse[]>(
            `/admin/api/v1/roles${buildListQueryString(pagination, sorting)}`
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
