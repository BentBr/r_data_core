import { describe, it, expect, vi, beforeEach } from 'vitest'
import { AuthClient } from './auth'
import type { LoginRequest, RefreshTokenRequest, LogoutRequest } from '@/types/schemas'

// Mock fetch and dependencies
const mockFetch = vi.fn()
global.fetch = mockFetch

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        token: 'test-token',
        refreshTokens: vi.fn(),
        logout: vi.fn(),
    }),
}))

vi.mock('@/utils/cookies', () => ({
    getRefreshToken: () => 'refresh-token',
}))

vi.mock('@/env-check', () => ({
    env: {
        apiBaseUrl: 'http://localhost:3000',
        enableApiLogging: false,
        devMode: false,
        defaultPageSize: 10,
    },
}))

describe('AuthClient', () => {
    let client: AuthClient

    beforeEach(() => {
        client = new AuthClient()
        vi.clearAllMocks()
    })

    describe('login', () => {
        it('should login successfully', async () => {
            const credentials: LoginRequest = {
                username: 'testuser',
                password: 'testpass',
            }

            const mockResponse = {
                status: 'Success',
                message: 'Login successful',
                data: {
                    access_token: 'access-token-123',
                    refresh_token: 'refresh-token-123',
                    user_uuid: '01234567-89ab-7def-8123-456789abcdef',
                    username: 'testuser',
                    role: 'admin',
                    access_expires_at: '2024-12-31T23:59:59Z',
                    refresh_expires_at: '2025-12-31T23:59:59Z',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.login(credentials)

            expect(result).toBeDefined()
            expect(result.access_token).toBe('access-token-123')
            expect(result.refresh_token).toBe('refresh-token-123')
            expect(result.username).toBe('testuser')
            expect(result.user_uuid).toBe('01234567-89ab-7def-8123-456789abcdef')
            expect(result.role).toBe('admin')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/auth/login'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify(credentials),
                })
            )
        })

        it('should handle login errors', async () => {
            const credentials: LoginRequest = {
                username: 'testuser',
                password: 'wrongpass',
            }

            const errorResponse = {
                status: 'Error',
                message: 'Invalid credentials',
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 401,
                json: async () => errorResponse,
            })

            await expect(client.login(credentials)).rejects.toThrow()
        })
    })

    describe('refreshToken', () => {
        it('should refresh token successfully', async () => {
            const request: RefreshTokenRequest = {
                refresh_token: 'refresh-token-123',
            }

            const mockResponse = {
                status: 'Success',
                message: 'Token refreshed',
                data: {
                    access_token: 'new-access-token',
                    refresh_token: 'new-refresh-token',
                    access_expires_at: '2024-12-31T23:59:59Z',
                    refresh_expires_at: '2025-12-31T23:59:59Z',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.refreshToken(request)

            expect(result).toBeDefined()
            expect(result.access_token).toBe('new-access-token')
            expect(result.refresh_token).toBe('new-refresh-token')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/auth/refresh'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify(request),
                })
            )
        })
    })

    describe('logout', () => {
        it('should logout successfully', async () => {
            const request: LogoutRequest = {
                refresh_token: 'refresh-token-123',
            }

            const mockResponse = {
                status: 'Success',
                message: 'Logged out successfully',
                data: null,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.logout(request)

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/auth/logout'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify(request),
                })
            )
        })
    })

    describe('revokeAllTokens', () => {
        it('should revoke all tokens successfully', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'All tokens revoked',
                data: { message: 'All tokens revoked' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.revokeAllTokens()

            expect(result).toBeDefined()
            expect(result.message).toBe('All tokens revoked')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/auth/revoke-all'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({}),
                })
            )
        })
    })
})
