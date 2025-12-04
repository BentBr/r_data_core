import { describe, it, expect, vi, beforeEach } from 'vitest'
import { EntitiesClient } from './entities'
import type { CreateEntityRequest, UpdateEntityRequest } from '@/types/schemas'

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

describe('EntitiesClient', () => {
    let client: EntitiesClient

    beforeEach(() => {
        client = new EntitiesClient()
        vi.clearAllMocks()
    })

    describe('getEntities', () => {
        it('should get entities with pagination', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [
                    {
                        entity_type: 'Customer',
                        field_data: {
                            uuid: '01234567-89ab-7def-8123-456789abcdef',
                            name: 'Test Customer',
                        },
                    },
                ],
                meta: {
                    pagination: {
                        total: 1,
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

            const result = await client.getEntities('Customer', 1, 10)

            expect(result.data).toBeDefined()
            expect(Array.isArray(result.data)).toBe(true)
            expect(result.data.length).toBe(1)
            expect(result.meta?.pagination).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/v1/Customer?page=1&per_page=10'),
                expect.any(Object)
            )
        })

        it('should include include parameter when provided', async () => {
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

            await client.getEntities('Customer', 1, 10, 'children')

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('&include=children'),
                expect.any(Object)
            )
        })
    })

    describe('getEntity', () => {
        it('should get a single entity', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: {
                    entity_type: 'Customer',
                    field_data: {
                        uuid: '01234567-89ab-7def-8123-456789abcdef',
                        name: 'Test Customer',
                    },
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getEntity(
                'Customer',
                '01234567-89ab-7def-8123-456789abcdef'
            )

            expect(result).toBeDefined()
            expect(result.entity_type).toBe('Customer')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/v1/Customer/01234567-89ab-7def-8123-456789abcdef'),
                expect.any(Object)
            )
        })
    })

    describe('createEntity', () => {
        it('should create an entity', async () => {
            const request: CreateEntityRequest = {
                entity_type: 'Customer',
                data: {
                    name: 'New Customer',
                },
            }

            const mockResponse = {
                status: 'Success',
                message: 'Created',
                data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    entity_type: 'Customer',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.createEntity('Customer', request)

            expect(result).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/v1/Customer'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ name: 'New Customer' }),
                })
            )
        })

        it('should include parent_uuid when provided', async () => {
            const request: CreateEntityRequest = {
                entity_type: 'Customer',
                data: { name: 'Child' },
                parent_uuid: '01234567-89ab-7def-8123-456789abcde0',
            }

            const mockResponse = {
                status: 'Success',
                message: 'Created',
                data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    entity_type: 'Customer',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            await client.createEntity('Customer', request)

            const callBody = JSON.parse(mockFetch.mock.calls[0][1].body as string)
            expect(callBody.parent_uuid).toBe('01234567-89ab-7def-8123-456789abcde0')
        })
    })

    describe('updateEntity', () => {
        it('should update an entity', async () => {
            const request: UpdateEntityRequest = {
                data: {
                    name: 'Updated Customer',
                },
            }

            const mockResponse = {
                status: 'Success',
                message: 'Updated',
                data: {
                    uuid: '01234567-89ab-7def-8123-456789abcdef',
                    entity_type: 'Customer',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.updateEntity(
                'Customer',
                '01234567-89ab-7def-8123-456789abcdef',
                request
            )

            expect(result).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/v1/Customer/01234567-89ab-7def-8123-456789abcdef'),
                expect.objectContaining({
                    method: 'PUT',
                })
            )
        })
    })

    describe('deleteEntity', () => {
        it('should delete an entity', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'Deleted',
                data: { message: 'Entity deleted' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.deleteEntity(
                'Customer',
                '01234567-89ab-7def-8123-456789abcdef'
            )

            expect(result).toBeDefined()
            expect(result.message).toBe('Entity deleted')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/v1/Customer/01234567-89ab-7def-8123-456789abcdef'),
                expect.objectContaining({
                    method: 'DELETE',
                })
            )
        })
    })

    describe('browseByPath', () => {
        it('should browse entities by path', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [
                    {
                        kind: 'folder' as const,
                        name: 'folder1',
                        path: '/folder1',
                        entity_uuid: '01234567-89ab-7def-8123-456789abcdef',
                        entity_type: 'Folder',
                        has_children: true,
                    },
                ],
                meta: {
                    pagination: {
                        total: 1,
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

            const result = await client.browseByPath('/test/path')

            expect(result.data).toBeDefined()
            expect(result.data[0].kind).toBe('folder')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/v1/entities/by-path?path='),
                expect.any(Object)
            )
        })
    })

    describe('queryEntities', () => {
        it('should query entities', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [
                    {
                        entity_type: 'Customer',
                        field_data: {
                            uuid: '01234567-89ab-7def-8123-456789abcdef',
                            name: 'Test',
                        },
                    },
                ],
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.queryEntities('Customer', { limit: 20, offset: 0 })

            expect(result).toBeDefined()
            expect(Array.isArray(result)).toBe(true)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/v1/entities/query'),
                expect.objectContaining({
                    method: 'POST',
                })
            )
        })
    })

    describe('listEntityVersions', () => {
        it('should list entity versions', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [
                    {
                        version_number: 1,
                        created_at: '2024-01-01T00:00:00Z',
                        created_by: '01234567-89ab-7def-8123-456789abcde0',
                        created_by_name: 'Test User',
                    },
                ],
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.listEntityVersions(
                'Customer',
                '01234567-89ab-7def-8123-456789abcdef'
            )

            expect(result).toBeDefined()
            expect(Array.isArray(result)).toBe(true)
            expect(result[0].version_number).toBe(1)
        })
    })

    describe('getEntityVersion', () => {
        it('should get a specific entity version', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: {
                    version_number: 1,
                    created_at: '2024-01-01T00:00:00Z',
                    created_by: '01234567-89ab-7def-8123-456789abcde0',
                    data: { name: 'Old Name' },
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getEntityVersion(
                'Customer',
                '01234567-89ab-7def-8123-456789abcdef',
                1
            )

            expect(result).toBeDefined()
            expect(result.version_number).toBe(1)
            expect(result.data).toEqual({ name: 'Old Name' })
        })
    })
})
