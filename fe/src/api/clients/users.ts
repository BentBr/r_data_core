import { z } from 'zod'
import { ApiResponseSchema, UserSchema } from '@/types/schemas'
import type { User } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class UsersClient extends BaseTypedHttpClient {
    async getUsers(limit?: number, offset = 0): Promise<User[]> {
        const pageSize = limit ?? this.getDefaultPageSize()
        return this.request(
            `/admin/api/v1/users?limit=${pageSize}&offset=${offset}`,
            ApiResponseSchema(z.array(UserSchema))
        )
    }
}

