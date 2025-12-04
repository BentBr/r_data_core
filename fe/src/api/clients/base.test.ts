import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { BaseTypedHttpClient } from './base'
import { ValidationError } from '../http-client'
import { z } from 'zod'

// Mock dependencies
const mockToken = 'test-token'
const mockRefreshToken = 'test-refresh-token'
const mockLogout = vi.fn()
const mockRefreshTokens = vi.fn()

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        token: mockToken,
        refreshTokens: mockRefreshTokens,
        logout: mockLogout,
    }),
}))

vi.mock('@/utils/cookies', () => ({
    getRefreshToken: () => mockRefreshToken,
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

// Create a test client class
class TestClient extends BaseTypedHttpClient {
    async testRequest<T>(endpoint: string, schema: z.ZodType<T>, options?: RequestInit) {
        return this.request(endpoint, schema, options)
    }

    async testPaginatedRequest<T>(endpoint: string, schema: z.ZodType<T>, options?: RequestInit) {
        return this.paginatedRequest(endpoint, schema, options)
    }
}

describe('BaseTypedHttpClient', () => {
    let client: TestClient
    let fetchSpy: ReturnType<typeof vi.fn>

    beforeEach(() => {
        client = new TestClient()
        fetchSpy = vi.fn()
        global.fetch = fetchSpy
        vi.clearAllMocks()
        mockRefreshTokens.mockResolvedValue(undefined)
    })

    afterEach(() => {
        vi.restoreAllMocks()
    })

    describe('request', () => {
        it('should make successful request with authentication', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number(), name: z.string() }),
            })

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: { id: 1, name: 'Test' },
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.testRequest('/test', schema)

            expect(fetchSpy).toHaveBeenCalledWith(
                'http://localhost:3000/test',
                expect.objectContaining({
                    headers: expect.objectContaining({
                        Authorization: `Bearer ${mockToken}`,
                        'Content-Type': 'application/json',
                    }),
                })
            )
            expect(result).toEqual(mockResponse.data)
        })

        it('should handle 401 unauthorized with token refresh', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: { id: 1 },
            }

            // First call returns 401
            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 401,
                json: async () => ({}),
            })

            // After refresh, retry succeeds
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.testRequest('/test', schema)

            expect(mockRefreshTokens).toHaveBeenCalled()
            expect(fetchSpy).toHaveBeenCalledTimes(2)
            expect(result).toEqual(mockResponse.data)
        })

        it('should handle 401 and logout if refresh fails', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            mockRefreshTokens.mockRejectedValueOnce(new Error('Refresh failed'))

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 401,
                json: async () => ({}),
            })

            await expect(client.testRequest('/test', schema)).rejects.toThrow(
                'Authentication required'
            )
            expect(mockLogout).toHaveBeenCalled()
        })

        it('should handle validation errors (422)', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                status: 'Error',
                message: 'Validation failed',
                violations: [{ field: 'name', message: 'Name is required', code: 'REQUIRED' }],
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 422,
                json: async () => errorResponse,
            })

            await expect(client.testRequest('/test', schema)).rejects.toThrow(ValidationError)
        })

        it('should handle 400 errors with validation-like structure', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                message: 'Json deserialize error: unknown variant `invalid`',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 400,
                json: async () => errorResponse,
            })

            await expect(client.testRequest('/test', schema)).rejects.toThrow(ValidationError)
        })

        it('should handle generic HTTP errors', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                status: 'Error',
                message: 'Server error',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 500,
                statusText: 'Internal Server Error',
                json: async () => errorResponse,
            })

            // The code checks errorData.status === 'Error' and errorData.message first
            // But if statusText is provided, it falls back to that
            // Since we're providing statusText, it will use the fallback format
            await expect(client.testRequest('/test', schema)).rejects.toThrow()
        })

        it('should handle response validation errors', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const invalidResponse = {
                status: 'Success',
                message: 'OK',
                data: { id: 'not-a-number' }, // Invalid type
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => invalidResponse,
            })

            await expect(client.testRequest('/test', schema)).rejects.toThrow(
                'Response validation failed'
            )
        })

        it('should handle DSL validate endpoint with fast-path', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ valid: z.boolean() }),
            })

            const mockResponse = {
                data: { valid: true },
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.testRequest('/dsl/validate', schema)

            expect(result).toEqual(mockResponse.data)
        })
    })

    describe('paginatedRequest', () => {
        it('should make successful paginated request', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.array(z.object({ id: z.number() })),
                meta: z
                    .object({
                        pagination: z.object({
                            total: z.number(),
                            page: z.number(),
                            per_page: z.number(),
                            total_pages: z.number(),
                            has_previous: z.boolean(),
                            has_next: z.boolean(),
                        }),
                    })
                    .optional(),
            })

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [{ id: 1 }, { id: 2 }],
                meta: {
                    pagination: {
                        total: 2,
                        page: 1,
                        per_page: 10,
                        total_pages: 1,
                        has_previous: false,
                        has_next: false,
                    },
                },
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.testPaginatedRequest('/test', schema)

            expect(result.data).toEqual(mockResponse.data)
            expect(result.meta).toEqual(mockResponse.meta)
        })

        it('should handle 401 in paginated request', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.array(z.object({ id: z.number() })),
            })

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [{ id: 1 }],
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 401,
                json: async () => ({}),
            })

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.testPaginatedRequest('/test', schema)

            expect(mockRefreshTokens).toHaveBeenCalled()
            expect(result.data).toEqual(mockResponse.data)
        })
    })

    describe('validateResponse', () => {
        it('should throw error for Error status responses', async () => {
            const schema = z.object({
                status: z.enum(['Success', 'Error']),
                message: z.string(),
                data: z.object({ id: z.number() }).optional(),
            })

            const errorResponse = {
                status: 'Error',
                message: 'Something went wrong',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => errorResponse,
            })

            await expect(client.testRequest('/test', schema)).rejects.toThrow(
                'Something went wrong'
            )
        })

        it('should handle null data responses', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.null(),
            })

            const response = {
                status: 'Success',
                message: 'OK',
                data: null,
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => response,
            })

            const result = await client.testRequest('/test', schema)

            expect(result).toEqual({ message: 'OK' })
        })
    })
})
