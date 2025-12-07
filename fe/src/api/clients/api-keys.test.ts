import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ApiKeysClient } from './api-keys'
import type { CreateApiKeyRequest, ReassignApiKeyRequest } from '@/types/schemas'

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
    buildApiUrl: (endpoint: string) => `http://localhost:3000${endpoint}`,
}))

describe('ApiKeysClient', () => {
    let client: ApiKeysClient

    beforeEach(() => {
        client = new ApiKeysClient()
        vi.clearAllMocks()
    })

    describe('getApiKeys', () => {
        it('should get API keys with pagination', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [],
                meta: {
                    pagination: {
                        total: 0,
                        page: 1,
                        per_page: 10,
                        total_pages: 1,
                        has_previous: false,
                        has_next: false,
                    },
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getApiKeys(1, 10)

            expect(result.data).toBeDefined()
            expect(Array.isArray(result.data)).toBe(true)
            expect(result.meta).toBeDefined()
            expect(result.meta?.pagination).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/api-keys?page=1&per_page=10'),
                expect.any(Object)
            )
        })

        it('should get API keys with sorting', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [],
                meta: {
                    pagination: {
                        total: 0,
                        page: 1,
                        per_page: 10,
                        total_pages: 1,
                        has_previous: false,
                        has_next: false,
                    },
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getApiKeys(1, 10, 'name', 'asc')

            expect(result.data).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/api-keys?page=1&per_page=10&sort_by=name&sort_order=asc'),
                expect.any(Object)
            )
        })

        it('should get API keys with descending sort', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [],
                meta: {
                    pagination: {
                        total: 0,
                        page: 1,
                        per_page: 10,
                        total_pages: 1,
                        has_previous: false,
                        has_next: false,
                    },
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getApiKeys(1, 10, 'created_at', 'desc')

            expect(result.data).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/api-keys?page=1&per_page=10&sort_by=created_at&sort_order=desc'),
                expect.any(Object)
            )
        })

        it('should not include sort parameters when not provided', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [],
                meta: {
                    pagination: {
                        total: 0,
                        page: 1,
                        per_page: 10,
                        total_pages: 1,
                        has_previous: false,
                        has_next: false,
                    },
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getApiKeys(1, 10, null, null)

            expect(result.data).toBeDefined()
            const url = mockFetch.mock.calls[0][0] as string
            expect(url).not.toContain('sort_by')
            expect(url).not.toContain('sort_order')
        })
    })

    describe('createApiKey', () => {
        it('should create API key', async () => {
            const request: CreateApiKeyRequest = {
                name: 'Test Key',
            }

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    name: 'Test Key',
                    description: undefined,
                    is_active: true,
                    created_at: '2024-01-01T00:00:00Z',
                    expires_at: null,
                    last_used_at: null,
                    created_by: '01234567-89ab-7def-8123-456789abcde0',
                    user_uuid: '01234567-89ab-7def-8123-456789abcde1',
                    published: true,
                    api_key: 'test-key-12345',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.createApiKey(request)

            expect(result).toBeDefined()
            expect(result.api_key).toBe('test-key-12345')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/api-keys'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify(request),
                })
            )
        })
    })

    describe('revokeApiKey', () => {
        it('should revoke API key', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'API key revoked',
                data: { message: 'API key revoked' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.revokeApiKey('test-uuid')

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/api-keys/test-uuid'),
                expect.objectContaining({
                    method: 'DELETE',
                })
            )
        })
    })

    describe('reassignApiKey', () => {
        it('should reassign API key', async () => {
            const request: ReassignApiKeyRequest = {
                user_uuid: 'user-uuid',
            }

            const mockResponse = {
                status: 'Success',
                message: 'API key reassigned',
                data: { message: 'API key reassigned' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.reassignApiKey('test-uuid', request)

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/api-keys/test-uuid/reassign'),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(request),
                })
            )
        })
    })
})
