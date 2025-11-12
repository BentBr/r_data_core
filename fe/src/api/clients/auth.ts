import { z } from 'zod'
import {
    ApiResponseSchema,
    LoginResponseSchema,
    RefreshTokenResponseSchema,
} from '@/types/schemas'
import type {
    LoginRequest,
    LoginResponse,
    RefreshTokenRequest,
    RefreshTokenResponse,
    LogoutRequest,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class AuthClient extends BaseTypedHttpClient {
    async login(credentials: LoginRequest): Promise<LoginResponse> {
        return this.request('/admin/api/v1/auth/login', ApiResponseSchema(LoginResponseSchema), {
            method: 'POST',
            body: JSON.stringify(credentials),
        })
    }

    async refreshToken(refreshTokenRequest: RefreshTokenRequest): Promise<RefreshTokenResponse> {
        return this.request(
            '/admin/api/v1/auth/refresh',
            ApiResponseSchema(RefreshTokenResponseSchema),
            {
                method: 'POST',
                body: JSON.stringify(refreshTokenRequest),
            }
        )
    }

    async logout(logoutRequest: LogoutRequest): Promise<{ message: string }> {
        const result = await this.request(
            '/admin/api/v1/auth/logout',
            ApiResponseSchema(z.null()),
            {
                method: 'POST',
                body: JSON.stringify(logoutRequest),
            }
        )
        return result as unknown as { message: string }
    }

    async revokeAllTokens(): Promise<{ message: string }> {
        return this.request(
            '/admin/api/v1/auth/revoke-all',
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'POST',
                body: JSON.stringify({}),
            }
        )
    }
}

