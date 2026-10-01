import { describe, it, expect, vi, beforeEach, afterEach, type Mock } from 'vitest'
import { z } from 'zod'
import { HttpClient, ValidationError, type ApiResponse } from './http-client'
import { HttpError } from './errors'

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

/** A zod schema for ApiResponse<T>, loose enough to accept whatever the tests hand it. */
function responseSchema<T>(dataSchema: z.ZodType<T>): z.ZodType<ApiResponse<T>> {
    return z.object({
        status: z.enum(['Success', 'Error']),
        message: z.string(),
        data: dataSchema.nullable().optional(),
        meta: z.unknown().optional(),
    }) as unknown as z.ZodType<ApiResponse<T>>
}

const idNameSchema = responseSchema(z.object({ id: z.number(), name: z.string().optional() }))

describe('HttpClient', () => {
    let client: HttpClient
    // `Mock`, not `ReturnType<typeof vi.fn>`: vitest 4 widened the latter to
    // Mock<Procedure | Constructable>, which no longer satisfies fetch's signature.
    let fetchSpy: Mock

    beforeEach(() => {
        client = new HttpClient()
        fetchSpy = vi.fn()
        global.fetch = fetchSpy
        vi.clearAllMocks()
        mockRefreshTokens.mockResolvedValue(undefined)
    })

    afterEach(() => {
        vi.restoreAllMocks()
    })

    describe('request', () => {
        it('makes a successful authenticated request and returns the parsed data', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: { id: 1, name: 'Test' },
            }
            fetchSpy.mockResolvedValueOnce({ ok: true, json: async () => mockResponse })

            const result = await client.request('/test', idNameSchema)

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

        it('refreshes the token and retries once on a 401', async () => {
            const mockResponse = { status: 'Success', message: 'OK', data: { id: 1 } }
            fetchSpy
                .mockResolvedValueOnce({ ok: false, status: 401, json: async () => ({}) })
                .mockResolvedValueOnce({ ok: true, json: async () => mockResponse })

            const result = await client.request('/test', idNameSchema)

            expect(mockRefreshTokens).toHaveBeenCalled()
            expect(fetchSpy).toHaveBeenCalledTimes(2)
            expect(result).toEqual(mockResponse.data)
        })

        it('logs out and throws when the refresh attempt itself fails', async () => {
            mockRefreshTokens.mockRejectedValueOnce(new Error('refresh failed'))
            fetchSpy.mockResolvedValueOnce({ ok: false, status: 401, json: async () => ({}) })

            await expect(client.request('/test', idNameSchema)).rejects.toThrow(
                'Authentication required'
            )
            expect(mockLogout).toHaveBeenCalled()
        })

        it('waits for an in-flight refresh instead of logging out on a concurrent 401', async () => {
            const mockResponse = { status: 'Success', message: 'OK', data: { id: 1 } }
            // Simulate another request already owning the refresh.

            ;(client as any).isRefreshing = true

            fetchSpy
                .mockResolvedValueOnce({ ok: false, status: 401, json: async () => ({}) })
                .mockResolvedValueOnce({ ok: true, json: async () => mockResponse })

            const result = await client.request('/test', idNameSchema)

            expect(result).toEqual(mockResponse.data)
            expect(mockRefreshTokens).toHaveBeenCalledTimes(1)
            expect(mockLogout).not.toHaveBeenCalled()
            expect(fetchSpy).toHaveBeenCalledTimes(2)
        })

        it('does not retry a 401 for the refresh endpoint itself, and logs out', async () => {
            fetchSpy.mockResolvedValueOnce({ ok: false, status: 401, json: async () => ({}) })

            await expect(
                client.request('/admin/api/v1/auth/refresh', idNameSchema)
            ).rejects.toThrow('Authentication required')
            expect(mockLogout).toHaveBeenCalled()
            expect(fetchSpy).toHaveBeenCalledTimes(1)
        })

        it('surfaces structured violations as a ValidationError on 422', async () => {
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

            await expect(client.request('/test', idNameSchema)).rejects.toThrow(ValidationError)
        })

        it('falls back to HttpError when a 422 body fails violation-schema parsing', async () => {
            const errorResponse = {
                status: 'Error',
                message: 'Validation failed',
                violations: 'not-an-array',
            }
            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 422,
                json: async () => errorResponse,
            })

            await expect(client.request('/test', idNameSchema)).rejects.toThrow(HttpError)
        })

        it('surfaces structured violations as a ValidationError on 400', async () => {
            const errorResponse = {
                status: 'Error',
                message: 'Validation failed',
                violations: [{ field: 'dsl', message: 'Invalid DSL', code: 'INVALID' }],
            }
            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 400,
                json: async () => errorResponse,
            })

            await expect(client.request('/test', idNameSchema)).rejects.toThrow(ValidationError)
        })

        it('maps backend { status: Error, message } bodies to HttpError with namespace/action', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 403,
                statusText: 'Forbidden',
                json: async () => ({ status: 'Error', message: 'Insufficient permissions' }),
            })

            let caught: unknown
            try {
                await client.request('/admin/api/v1/users', idNameSchema, { method: 'POST' })
            } catch (error) {
                caught = error
            }

            expect(caught).toBeInstanceOf(HttpError)
            expect((caught as HttpError).statusCode).toBe(403)
            expect((caught as HttpError).namespace).toBe('user')
            expect((caught as HttpError).action).toBe('create')
            expect((caught as HttpError).message).toBe('Insufficient permissions')
        })

        it('falls back to the error field or statusText when no standard message exists', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 403,
                statusText: 'Forbidden',
                json: async () => ({ error: 'Access denied' }),
            })

            await expect(client.request('/admin/api/v1/api-keys', idNameSchema)).rejects.toThrow(
                'Access denied'
            )
        })

        it('throws HttpError using statusText when the error body cannot be parsed at all', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 500,
                statusText: 'Internal Server Error',
                json: async () => {
                    throw new Error('invalid JSON')
                },
            })

            let caught: unknown
            try {
                await client.request('/admin/api/v1/workflows', idNameSchema, { method: 'DELETE' })
            } catch (error) {
                caught = error
            }

            expect(caught).toBeInstanceOf(HttpError)
            expect((caught as HttpError).statusCode).toBe(500)
            expect((caught as HttpError).namespace).toBe('workflow')
            expect((caught as HttpError).action).toBe('delete')
            expect((caught as HttpError).message).toBe('Internal Server Error')
        })

        it('rejects with a response-validation error when the success body does not match the schema', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: { id: 'not-a-number' },
                }),
            })

            await expect(client.request('/test', idNameSchema)).rejects.toThrow(
                /Response validation failed/
            )
        })

        it('treats a 409 as an expected error without throwing an unrelated type', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: false,
                status: 409,
                statusText: 'Conflict',
                json: async () => ({ status: 'Error', message: 'Already exists' }),
            })

            await expect(
                client.request('/admin/api/v1/users', idNameSchema, { method: 'POST' })
            ).rejects.toThrow('Already exists')
        })
    })

    describe('validateResponse (exercised via request)', () => {
        it('throws the server message when status is Error', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({ status: 'Error', message: 'Something went wrong' }),
            })

            await expect(client.request('/test', idNameSchema)).rejects.toThrow(
                'Something went wrong'
            )
        })

        it('returns { message } for responses with null data (e.g. logout)', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({ status: 'Success', message: 'Logged out', data: null }),
            })

            const result = await client.request('/test', idNameSchema)
            expect(result).toEqual({ message: 'Logged out' })
        })

        it('throws "No data in successful response" when data is missing entirely', async () => {
            fetchSpy.mockResolvedValueOnce({
                ok: true,
                json: async () => ({ status: 'Success', message: 'OK' }),
            })

            await expect(client.request('/test', idNameSchema)).rejects.toThrow(
                'No data in successful response'
            )
        })
    })
})
