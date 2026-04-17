import { describe, it, expect, vi, beforeEach } from 'vitest'
import { DslClient } from './dsl'
import type { DslStep } from '@/types/schemas'

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

const mockOptionsResponse = {
    types: [{ type: 'format', fields: [{ name: 'source', type: 'object', required: true }] }],
}

const mockDslStep: DslStep = {
    from: {
        type: 'entity',
        entity_definition: 'product',
        mapping: { uuid: 'uuid', name: 'title' },
    },
    to: {
        type: 'entity',
        entity_definition: 'catalog_item',
        mode: 'create_or_update',
        mapping: { uuid: 'uuid', title: 'name' },
    },
    transform: {
        type: 'none',
    },
}

describe('DslClient', () => {
    let client: DslClient

    beforeEach(() => {
        client = new DslClient()
        vi.clearAllMocks()
    })

    describe('getDslFromOptions', () => {
        it('should get DSL from options', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockOptionsResponse,
                }),
            })

            const result = await client.getDslFromOptions()

            expect(result).toBeDefined()
            expect(result.types).toBeDefined()
            expect(Array.isArray(result.types)).toBe(true)
            expect(result.types[0].type).toBe('format')
            expect(result.types[0].fields[0].name).toBe('source')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/dsl/from/options'),
                expect.any(Object)
            )
        })

        it('should throw on error response', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 500,
                json: async () => ({ status: 'Error', message: 'Internal Server Error' }),
            })

            await expect(client.getDslFromOptions()).rejects.toThrow()
        })
    })

    describe('getDslToOptions', () => {
        it('should get DSL to options', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockOptionsResponse,
                }),
            })

            const result = await client.getDslToOptions()

            expect(result).toBeDefined()
            expect(result.types).toBeDefined()
            expect(Array.isArray(result.types)).toBe(true)
            expect(result.types[0].type).toBe('format')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/dsl/to/options'),
                expect.any(Object)
            )
        })

        it('should return an empty types array when no options exist', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: { types: [] },
                }),
            })

            const result = await client.getDslToOptions()

            expect(result.types).toHaveLength(0)
        })
    })

    describe('getDslTransformOptions', () => {
        it('should get DSL transform options', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockOptionsResponse,
                }),
            })

            const result = await client.getDslTransformOptions()

            expect(result).toBeDefined()
            expect(result.types).toBeDefined()
            expect(Array.isArray(result.types)).toBe(true)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/dsl/transform/options'),
                expect.any(Object)
            )
        })

        it('should return multiple transform types', async () => {
            const multiTypeResponse = {
                types: [
                    { type: 'arithmetic', fields: [{ name: 'target', type: 'string', required: true }] },
                    { type: 'concat', fields: [{ name: 'target', type: 'string', required: true }] },
                    { type: 'none', fields: [] },
                ],
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: multiTypeResponse,
                }),
            })

            const result = await client.getDslTransformOptions()

            expect(result.types).toHaveLength(3)
            expect(result.types[0].type).toBe('arithmetic')
            expect(result.types[2].type).toBe('none')
        })
    })

    describe('validateDsl', () => {
        it('should validate a valid DSL step array and return true', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: { valid: true },
                }),
            })

            const steps: DslStep[] = [mockDslStep]
            const result = await client.validateDsl(steps)

            expect(result).toBeDefined()
            expect(result.valid).toBe(true)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/dsl/validate'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ steps }),
                })
            )
        })

        it('should return false for an invalid DSL step array', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: { valid: false },
                }),
            })

            const steps: DslStep[] = [mockDslStep]
            const result = await client.validateDsl(steps)

            expect(result.valid).toBe(false)
        })

        it('should throw on 422 for a structurally malformed request', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 422,
                json: async () => ({
                    status: 'Error',
                    message: 'Unprocessable Entity',
                }),
            })

            await expect(client.validateDsl([])).rejects.toThrow()
        })

        it('should send steps wrapped in a steps object', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: { valid: true },
                }),
            })

            const steps: DslStep[] = [mockDslStep]
            await client.validateDsl(steps)

            const callBody = JSON.parse(mockFetch.mock.calls[0][1].body as string) as {
                steps: DslStep[]
            }
            expect(callBody).toHaveProperty('steps')
            expect(Array.isArray(callBody.steps)).toBe(true)
            expect(callBody.steps).toHaveLength(1)
        })
    })
})
