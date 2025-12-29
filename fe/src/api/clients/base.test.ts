import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { BaseTypedHttpClient } from './base'
import { ValidationError } from '../http-client'
import { HttpError } from '../errors'
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

// Import ApiResponse type from base for typing
import type { ApiResponse } from './base'

// Create a test client class
class TestClient extends BaseTypedHttpClient {
    async testRequest<T>(
        endpoint: string,
        schema: z.ZodType<ApiResponse<T>>,
        options?: RequestInit
    ) {
        return this.request(endpoint, schema, options)
    }

    async testPaginatedRequest<T>(
        endpoint: string,
        schema: z.ZodType<ApiResponse<T>>,
        options?: RequestInit
    ) {
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

        it('should throw HttpError with 403 status code and context', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                status: 'Error',
                message: 'Insufficient permissions',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 403,
                statusText: 'Forbidden',
                json: async () => errorResponse,
            })

            let caughtError: unknown
            try {
                await client.testRequest('/admin/api/v1/users', schema, { method: 'POST' })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(403)
            expect((caughtError as HttpError).namespace).toBe('user')
            expect((caughtError as HttpError).action).toBe('create')
            expect((caughtError as HttpError).message).toBe('Insufficient permissions')
        })

        it('should throw HttpError with correct namespace extraction', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                status: 'Error',
                message: 'Permission denied',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 403,
                statusText: 'Forbidden',
                json: async () => errorResponse,
            })

            let caughtError: unknown
            try {
                await client.testRequest('/admin/api/v1/roles', schema, { method: 'GET' })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(403)
            expect((caughtError as HttpError).namespace).toBe('role')
            expect((caughtError as HttpError).action).toBe('read')
        })

        it('should throw HttpError with fallback message when error format is different', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                error: 'Access denied',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 403,
                statusText: 'Forbidden',
                json: async () => errorResponse,
            })

            let caughtError: unknown
            try {
                await client.testRequest('/admin/api/v1/api-keys', schema, { method: 'PUT' })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(403)
            expect((caughtError as HttpError).namespace).toBe('api_key')
            expect((caughtError as HttpError).action).toBe('update')
            expect((caughtError as HttpError).message).toBe('Access denied')
        })

        it('should throw HttpError when response parsing fails', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 403,
                statusText: 'Forbidden',
                json: async () => {
                    throw new Error('Invalid JSON')
                },
            })

            let caughtError: unknown
            try {
                await client.testRequest('/admin/api/v1/workflows', schema, { method: 'DELETE' })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(403)
            expect((caughtError as HttpError).namespace).toBe('workflow')
            expect((caughtError as HttpError).action).toBe('delete')
            expect((caughtError as HttpError).message).toBe('Forbidden')
        })

        it('should throw HttpError with 409 conflict status code and context', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                status: 'Error',
                message: 'A user with this username already exists',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 409,
                statusText: 'Conflict',
                json: async () => errorResponse,
            })

            let caughtError: unknown
            try {
                await client.testRequest('/admin/api/v1/users', schema, { method: 'POST' })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(409)
            expect((caughtError as HttpError).namespace).toBe('user')
            expect((caughtError as HttpError).action).toBe('create')
            expect((caughtError as HttpError).message).toBe(
                'A user with this username already exists'
            )
        })

        it('should throw HttpError with 409 conflict for entity definitions', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                status: 'Error',
                message: 'An entity definition with this type already exists',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 409,
                statusText: 'Conflict',
                json: async () => errorResponse,
            })

            let caughtError: unknown
            try {
                await client.testRequest('/admin/api/v1/entity-definitions', schema, {
                    method: 'POST',
                })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(409)
            expect((caughtError as HttpError).namespace).toBe('entity_definition')
            expect((caughtError as HttpError).action).toBe('create')
            expect((caughtError as HttpError).message).toBe(
                'An entity definition with this type already exists'
            )
        })

        it('should throw HttpError with 409 conflict for API keys', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                status: 'Error',
                message: 'An API key with this name already exists',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 409,
                statusText: 'Conflict',
                json: async () => errorResponse,
            })

            let caughtError: unknown
            try {
                await client.testRequest('/admin/api/v1/api-keys', schema, { method: 'POST' })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(409)
            expect((caughtError as HttpError).namespace).toBe('api_key')
            expect((caughtError as HttpError).action).toBe('create')
            expect((caughtError as HttpError).message).toBe(
                'An API key with this name already exists'
            )
        })

        it('should not log 409 errors as console.error (only in devMode as console.log)', async () => {
            const schema = z.object({
                status: z.literal('Success'),
                message: z.string(),
                data: z.object({ id: z.number() }),
            })

            const errorResponse = {
                status: 'Error',
                message: 'Conflict',
            }

            const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
            const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {})

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 409,
                statusText: 'Conflict',
                json: async () => errorResponse,
            })

            try {
                await client.testRequest('/admin/api/v1/users', schema, { method: 'POST' })
            } catch {
                // Expected
            }

            // 409 is an expected error, should not be logged as console.error
            // (unless in devMode where it would be logged as console.log)
            expect(consoleErrorSpy).not.toHaveBeenCalledWith(
                '[API] HTTP Error Response:',
                expect.anything()
            )

            consoleErrorSpy.mockRestore()
            consoleLogSpy.mockRestore()
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
