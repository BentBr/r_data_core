import type { AdminLoginResponse } from '@/types/generated/AdminLoginResponse'
import type { RefreshTokenResponse } from '@/types/generated/RefreshTokenResponse'
import type { LoginRequest, RefreshTokenRequest, LogoutRequest } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class AuthClient extends BaseTypedHttpClient {
    async login(credentials: LoginRequest): Promise<AdminLoginResponse> {
        return this.request<AdminLoginResponse>('/admin/api/v1/auth/login', {
            method: 'POST',
            body: JSON.stringify(credentials),
        })
    }

    async refreshToken(refreshTokenRequest: RefreshTokenRequest): Promise<RefreshTokenResponse> {
        return this.request<RefreshTokenResponse>('/admin/api/v1/auth/refresh', {
            method: 'POST',
            body: JSON.stringify(refreshTokenRequest),
        })
    }

    async logout(logoutRequest: LogoutRequest): Promise<{ message: string }> {
        return this.request<{ message: string }>('/admin/api/v1/auth/logout', {
            method: 'POST',
            body: JSON.stringify(logoutRequest),
        })
    }

    async revokeAllTokens(): Promise<{ message: string }> {
        return this.request<{ message: string }>('/admin/api/v1/auth/revoke-all', {
            method: 'POST',
            body: JSON.stringify({}),
        })
    }

    async getUserPermissions(): Promise<{
        is_super_admin: boolean
        permissions: string[]
        allowed_routes: string[]
    }> {
        return this.request<{
            is_super_admin: boolean
            permissions: string[]
            allowed_routes: string[]
        }>('/admin/api/v1/auth/permissions')
    }

    async forgotPassword(email: string): Promise<void> {
        await this.request<{ message: string }>('/admin/api/v1/auth/forgot-password', {
            method: 'POST',
            body: JSON.stringify({ email }),
        })
    }

    async resetPassword(token: string, newPassword: string): Promise<void> {
        await this.request<{ message: string }>('/admin/api/v1/auth/reset-password', {
            method: 'POST',
            body: JSON.stringify({ token, new_password: newPassword }),
        })
    }
}
