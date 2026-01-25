import { z } from 'zod'
import { ApiResponseSchema, PaginatedApiResponseSchema, UserResponseSchema } from '@/types/schemas'
import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class UsersClient extends BaseTypedHttpClient {
    async getUsers(
        page = 1,
        itemsPerPage = 20,
        sortBy?: string | null,
        sortOrder?: 'asc' | 'desc' | null
    ): Promise<{
        data: UserResponse[]
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
        let url = `/admin/api/v1/users?page=${page}&per_page=${itemsPerPage}`
        if (sortBy && sortOrder) {
            url += `&sort_by=${sortBy}&sort_order=${sortOrder}`
        }
        const response = await this.paginatedRequest(
            url,
            PaginatedApiResponseSchema(z.array(UserResponseSchema))
        )
        return response
    }

    async getUser(uuid: string): Promise<UserResponse> {
        return this.request(`/admin/api/v1/users/${uuid}`, ApiResponseSchema(UserResponseSchema))
    }

    async createUser(data: CreateUserRequest): Promise<UserResponse> {
        return this.request('/admin/api/v1/users', ApiResponseSchema(UserResponseSchema), {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async updateUser(uuid: string, data: UpdateUserRequest): Promise<UserResponse> {
        return this.request(`/admin/api/v1/users/${uuid}`, ApiResponseSchema(UserResponseSchema), {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async deleteUser(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/users/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async getUserRoles(uuid: string): Promise<string[]> {
        return this.request(
            `/admin/api/v1/users/${uuid}/roles`,
            ApiResponseSchema(z.array(z.string()))
        )
    }

    async assignRolesToUser(uuid: string, roleUuids: string[]): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/users/${uuid}/roles`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'PUT',
                body: JSON.stringify(roleUuids),
            }
        )
    }
}
