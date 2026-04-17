import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { BaseTypedHttpClient } from './base'
import { ValidationError } from '../http-client'
import { HttpError } from '../errors'

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
    async testRequest<T>(endpoint: string, options?: RequestInit) {
        return this.request<T>(endpoint, options)
    }

    async testPaginatedRequest<T>(endpoint: string, options?: RequestInit) {
        return this.paginatedRequest<T>(endpoint, options)
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
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: { id: 1, name: 'Test' },
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.testRequest<{ id: number; name: string }>('/test')

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

            const result = await client.testRequest<{ id: number }>('/test')

            expect(mockRefreshTokens).toHaveBeenCalled()
            expect(fetchSpy).toHaveBeenCalledTimes(2)
            expect(result).toEqual(mockResponse.data)
        })

        it('should handle 401 and logout if refresh fails', async () => {
            mockRefreshTokens.mockRejectedValueOnce(new Error('Refresh failed'))

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 401,
                json: async () => ({}),
            })

            await expect(client.testRequest('/test')).rejects.toThrow('Authentication required')
            expect(mockLogout).toHaveBeenCalled()
        })

        it('should handle validation errors (422)', async () => {
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

            await expect(client.testRequest('/test')).rejects.toThrow(ValidationError)
        })

        it('should handle 400 errors with validation-like structure', async () => {
            const errorResponse = {
                message: 'Json deserialize error: unknown variant `invalid`',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 400,
                json: async () => errorResponse,
            })

            await expect(client.testRequest('/test')).rejects.toThrow(ValidationError)
        })

        it('should handle generic HTTP errors', async () => {
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
            await expect(client.testRequest('/test')).rejects.toThrow()
        })

        it('should throw HttpError with 403 status code and context', async () => {
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
                await client.testRequest('/admin/api/v1/users', { method: 'POST' })
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
                await client.testRequest('/admin/api/v1/roles', { method: 'GET' })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(403)
            expect((caughtError as HttpError).namespace).toBe('role')
            expect((caughtError as HttpError).action).toBe('read')
        })

        it('should throw HttpError with fallback message when error format is different', async () => {
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
                await client.testRequest('/admin/api/v1/api-keys', { method: 'PUT' })
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
                await client.testRequest('/admin/api/v1/workflows', { method: 'DELETE' })
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
                await client.testRequest('/admin/api/v1/users', { method: 'POST' })
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
                await client.testRequest('/admin/api/v1/entity-definitions', {
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
                await client.testRequest('/admin/api/v1/api-keys', { method: 'POST' })
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
                await client.testRequest('/admin/api/v1/users', { method: 'POST' })
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

        it('should wait for ongoing refresh instead of logging out on concurrent 401', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: { id: 1 },
            }

            // Simulate isRefreshing already being true (another request is refreshing)
            // Access the private field to simulate the concurrent state

            ;(client as any).isRefreshing = true

            // First call returns 401, retry (second call) returns success
            fetchSpy
                .mockResolvedValueOnce({
                    ok: false,
                    status: 401,
                    json: async () => ({}),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockResponse,
                })

            const result = await client.testRequest<{ id: number }>('/test')

            expect(result).toEqual(mockResponse.data)
            // Should still call refreshTokens (waits via auth store's shared promise)
            expect(mockRefreshTokens).toHaveBeenCalledTimes(1)
            // Should NOT call logout — the old behavior would skip refresh and logout
            expect(mockLogout).not.toHaveBeenCalled()
            // Should have retried the request (2 fetch calls total)
            expect(fetchSpy).toHaveBeenCalledTimes(2)
        })

        it('should throw HttpError with 404 status code', async () => {
            const errorResponse = {
                status: 'Error',
                message: 'Entity not found',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => errorResponse,
            })

            let caughtError: unknown
            try {
                await client.testRequest('/admin/api/v1/entities/123', { method: 'GET' })
            } catch (error) {
                caughtError = error
            }

            expect(caughtError).toBeInstanceOf(HttpError)
            expect((caughtError as HttpError).statusCode).toBe(404)
            expect((caughtError as HttpError).namespace).toBe('entity')
            expect((caughtError as HttpError).action).toBe('read')
            expect((caughtError as HttpError).message).toBe('Entity not found')
        })

        it('should handle DSL validate endpoint', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: { valid: true },
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.testRequest<{ valid: boolean }>('/dsl/validate')

            expect(result).toEqual(mockResponse.data)
        })
    })

    describe('paginatedRequest', () => {
        it('should make successful paginated request', async () => {
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

            const result = await client.testPaginatedRequest<Array<{ id: number }>>('/test')

            expect(result.data).toEqual(mockResponse.data)
            expect(result.meta).toEqual(mockResponse.meta)
        })

        it('should handle 401 in paginated request with token refresh', async () => {
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

            const result = await client.testPaginatedRequest<Array<{ id: number }>>('/test')

            expect(mockRefreshTokens).toHaveBeenCalled()
            expect(mockLogout).not.toHaveBeenCalled()
            expect(result.data).toEqual(mockResponse.data)
        })

        it('should handle 401 and logout if refresh fails in paginated request', async () => {
            mockRefreshTokens.mockRejectedValueOnce(new Error('Refresh failed'))

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 401,
                json: async () => ({}),
            })

            await expect(client.testPaginatedRequest('/test')).rejects.toThrow(
                'Authentication required'
            )
            expect(mockLogout).toHaveBeenCalled()
        })

        it('should wait for ongoing refresh on concurrent 401 in paginated request', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [{ id: 1 }],
            }

            // Simulate isRefreshing already being true

            ;(client as any).isRefreshing = true

            fetchSpy
                .mockResolvedValueOnce({
                    ok: false,
                    status: 401,
                    json: async () => ({}),
                })
                .mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockResponse,
                })

            const result = await client.testPaginatedRequest<Array<{ id: number }>>('/test')

            expect(result.data).toEqual(mockResponse.data)
            expect(mockRefreshTokens).toHaveBeenCalledTimes(1)
            expect(mockLogout).not.toHaveBeenCalled()
            expect(fetchSpy).toHaveBeenCalledTimes(2)
        })

        it('should handle 404 error in paginated request', async () => {
            const errorResponse = {
                status: 'Error',
                message: 'Resource not found',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => errorResponse,
            })

            await expect(client.testPaginatedRequest('/test')).rejects.toThrow('Resource not found')
            expect(mockLogout).not.toHaveBeenCalled()
        })

        it('should handle 403 error in paginated request', async () => {
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

            await expect(client.testPaginatedRequest('/test')).rejects.toThrow(
                'Insufficient permissions'
            )
            expect(mockLogout).not.toHaveBeenCalled()
        })

        it('should handle 500 error in paginated request', async () => {
            const errorResponse = {
                status: 'Error',
                message: 'Internal server error',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 500,
                statusText: 'Internal Server Error',
                json: async () => errorResponse,
            })

            await expect(client.testPaginatedRequest('/test')).rejects.toThrow(
                'Internal server error'
            )
        })

        it('should handle 422 validation error in paginated request', async () => {
            const errorResponse = {
                status: 'Error',
                message: 'Validation failed',
                violations: [{ field: 'page', message: 'Must be positive', code: 'INVALID' }],
            }

            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 422,
                statusText: 'Unprocessable Entity',
                json: async () => errorResponse,
            })

            await expect(client.testPaginatedRequest('/test')).rejects.toThrow(ValidationError)
        })
    })

    describe('validateResponse', () => {
        it('should throw error for Error status responses', async () => {
            const errorResponse = {
                status: 'Error',
                message: 'Something went wrong',
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => errorResponse,
            })

            await expect(client.testRequest('/test')).rejects.toThrow('Something went wrong')
        })

        it('should handle null data responses', async () => {
            const response = {
                status: 'Success',
                message: 'OK',
                data: null,
            }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => response,
            })

            const result = await client.testRequest('/test')

            expect(result).toEqual({ message: 'OK' })
        })
    })

    describe('validateResponse edge cases (post-refactor)', () => {
        // After removing Zod, validateResponse uses `as` cast.
        // These tests document what actually happens with unexpected shapes.

        it('should throw when response has status Error', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Error',
                    message: 'Something went wrong',
                    data: null,
                }),
            })

            await expect(client.testRequest('/test')).rejects.toThrow('Something went wrong')
        })

        it('should throw when response has no data and status is Success', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    // data field missing entirely
                }),
            })

            await expect(client.testRequest('/test')).rejects.toThrow(
                'No data in successful response'
            )
        })

        it('should throw when response has undefined data', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: undefined,
                }),
            })

            await expect(client.testRequest('/test')).rejects.toThrow(
                'No data in successful response'
            )
        })

        it('should pass through data even if shape is unexpected (no runtime validation)', async () => {
            // With Zod removed, the client trusts the backend. If the shape is wrong,
            // it passes through — the error surfaces later in the UI, not here.
            const weirdData = { unexpected_field: 42, nested: { deep: true } }

            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: weirdData,
                }),
            })

            const result = await client.testRequest('/test')
            expect(result).toEqual(weirdData)
        })

        it('should handle response with missing status field gracefully', async () => {
            // When status is undefined, `response.status === 'Error'` is false,
            // and the code falls through to check data.
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    message: 'OK',
                    data: { id: 1 },
                }),
            })

            const result = await client.testRequest('/test')
            expect(result).toEqual({ id: 1 })
        })

        it('should handle empty object response', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({}),
            })

            await expect(client.testRequest('/test')).rejects.toThrow(
                'No data in successful response'
            )
        })

        it('should handle paginated response with missing meta', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: [{ id: 1 }],
                    // meta missing
                }),
            })

            const result = await client.testPaginatedRequest('/test')
            expect(result.data).toEqual([{ id: 1 }])
            expect(result.meta).toBeUndefined()
        })

        it('should handle paginated response with null meta', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: [{ id: 1 }],
                    meta: null,
                }),
            })

            const result = await client.testPaginatedRequest('/test')
            expect(result.data).toEqual([{ id: 1 }])
            expect(result.meta).toBeUndefined()
        })
    })
})
