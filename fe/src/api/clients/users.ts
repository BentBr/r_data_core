import type { UserResponse } from '@/types/generated/UserResponse'
import type { PaginationQuery } from '@/types/generated/PaginationQuery'
import type { SortingQuery } from '@/types/generated/SortingQuery'
import type { CreateUserRequest, UpdateUserRequest, ResponseMeta } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'
import { buildListQueryString } from './query'

export class UsersClient extends BaseTypedHttpClient {
    async getUsers(
        pagination: PaginationQuery,
        sorting?: SortingQuery | null
    ): Promise<{ data: UserResponse[]; meta?: ResponseMeta }> {
        return this.paginatedRequest<UserResponse[]>(
            `/admin/api/v1/users${buildListQueryString(pagination, sorting)}`
        )
    }

    async getUser(uuid: string): Promise<UserResponse> {
        return this.request<UserResponse>(`/admin/api/v1/users/${uuid}`)
    }

    async createUser(data: CreateUserRequest): Promise<UserResponse> {
        return this.request<UserResponse>('/admin/api/v1/users', {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async updateUser(uuid: string, data: UpdateUserRequest): Promise<UserResponse> {
        return this.request<UserResponse>(`/admin/api/v1/users/${uuid}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async deleteUser(uuid: string): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/users/${uuid}`, {
            method: 'DELETE',
        })
    }

    async getUserRoles(uuid: string): Promise<string[]> {
        return this.request<string[]>(`/admin/api/v1/users/${uuid}/roles`)
    }

    async assignRolesToUser(uuid: string, roleUuids: string[]): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/users/${uuid}/roles`, {
            method: 'PUT',
            body: JSON.stringify(roleUuids),
        })
    }
}
