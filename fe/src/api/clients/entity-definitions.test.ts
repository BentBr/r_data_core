import { describe, it, expect, vi, beforeEach } from 'vitest'
import { EntityDefinitionsClient } from './entity-definitions'
import type {
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
    EntityDefinition,
} from '@/types/schemas'

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

const mockField = {
    name: 'title',
    display_name: 'Title',
    field_type: 'String' as const,
    description: null,
    required: true,
    indexed: false,
    filterable: true,
    unique: false,
    default_value: null,
    constraints: null,
    ui_settings: {},
}

const mockEntityDefinition: EntityDefinition = {
    uuid: 'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
    entity_type: 'product',
    display_name: 'Product',
    description: 'A product entity',
    group_name: 'catalog',
    allow_children: false,
    icon: 'mdi-package',
    fields: [mockField],
    published: true,
    created_at: '2024-01-15T10:00:00Z',
    updated_at: '2024-06-01T08:30:00Z',
}

describe('EntityDefinitionsClient', () => {
    let client: EntityDefinitionsClient

    beforeEach(() => {
        client = new EntityDefinitionsClient()
        vi.clearAllMocks()
    })

    describe('getEntityDefinitions', () => {
        it('should get entity definitions with pagination', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [mockEntityDefinition],
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

            const result = await client.getEntityDefinitions(10, 0)

            expect(result.data).toBeDefined()
            expect(Array.isArray(result.data)).toBe(true)
            expect(result.data[0].entity_type).toBe('product')
            expect(result.meta).toBeDefined()
            expect(result.meta?.pagination?.total).toBe(1)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/entity-definitions?limit=10&offset=0'),
                expect.any(Object)
            )
        })

        it('should respect limit and offset parameters', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [],
                meta: {
                    pagination: {
                        total: 50,
                        page: 3,
                        per_page: 20,
                        total_pages: 3,
                        has_previous: true,
                        has_next: false,
                    },
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getEntityDefinitions(20, 40)

            expect(result.meta?.pagination?.has_previous).toBe(true)
            expect(result.meta?.pagination?.has_next).toBe(false)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/entity-definitions?limit=20&offset=40'),
                expect.any(Object)
            )
        })
    })

    describe('getEntityDefinition', () => {
        it('should get a single entity definition by uuid', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: mockEntityDefinition,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getEntityDefinition('a1b2c3d4-e5f6-7890-abcd-ef1234567890')

            expect(result).toBeDefined()
            expect(result.uuid).toBe('a1b2c3d4-e5f6-7890-abcd-ef1234567890')
            expect(result.entity_type).toBe('product')
            expect(result.display_name).toBe('Product')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/entity-definitions/a1b2c3d4-e5f6-7890-abcd-ef1234567890'
                ),
                expect.any(Object)
            )
        })

        it('should throw on 404 for unknown uuid', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                json: async () => ({ status: 'Error', message: 'Not found' }),
            })

            await expect(
                client.getEntityDefinition('00000000-0000-0000-0000-000000000000')
            ).rejects.toThrow()
        })
    })

    describe('createEntityDefinition', () => {
        it('should create a new entity definition and return uuid', async () => {
            const request: CreateEntityDefinitionRequest = {
                entity_type: 'category',
                display_name: 'Category',
                description: 'A category entity',
                allow_children: true,
                fields: [],
                published: false,
            }

            const mockResponse = {
                status: 'Success',
                message: 'Created',
                data: { uuid: 'b2c3d4e5-f6a7-8901-bcde-f12345678901' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.createEntityDefinition(request)

            expect(result).toBeDefined()
            expect(result.uuid).toBe('b2c3d4e5-f6a7-8901-bcde-f12345678901')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/entity-definitions'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify(request),
                })
            )
        })

        it('should throw on 422 for invalid request body', async () => {
            const request: CreateEntityDefinitionRequest = {
                entity_type: '',
                display_name: '',
                allow_children: false,
                fields: [],
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 422,
                json: async () => ({
                    status: 'Error',
                    message: 'Validation failed',
                }),
            })

            await expect(client.createEntityDefinition(request)).rejects.toThrow()
        })
    })

    describe('updateEntityDefinition', () => {
        it('should update an entity definition and return uuid', async () => {
            const request: UpdateEntityDefinitionRequest = {
                entity_type: 'product',
                display_name: 'Product (Updated)',
                allow_children: false,
                fields: [mockField],
                published: true,
            }

            const mockResponse = {
                status: 'Success',
                message: 'Updated',
                data: { uuid: 'a1b2c3d4-e5f6-7890-abcd-ef1234567890' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.updateEntityDefinition(
                'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
                request
            )

            expect(result).toBeDefined()
            expect(result.uuid).toBe('a1b2c3d4-e5f6-7890-abcd-ef1234567890')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/entity-definitions/a1b2c3d4-e5f6-7890-abcd-ef1234567890'
                ),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(request),
                })
            )
        })

        it('should throw on 404 when entity definition not found', async () => {
            const request: UpdateEntityDefinitionRequest = {
                entity_type: 'ghost',
                display_name: 'Ghost',
                allow_children: false,
                fields: [],
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                json: async () => ({ status: 'Error', message: 'Not found' }),
            })

            await expect(
                client.updateEntityDefinition('00000000-0000-0000-0000-000000000000', request)
            ).rejects.toThrow()
        })
    })

    describe('deleteEntityDefinition', () => {
        it('should delete an entity definition', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'Deleted',
                data: { message: 'Entity definition deleted' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.deleteEntityDefinition(
                'a1b2c3d4-e5f6-7890-abcd-ef1234567890'
            )

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/entity-definitions/a1b2c3d4-e5f6-7890-abcd-ef1234567890'
                ),
                expect.objectContaining({
                    method: 'DELETE',
                })
            )
        })

        it('should throw on 404 for non-existent entity definition', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                json: async () => ({ status: 'Error', message: 'Not found' }),
            })

            await expect(
                client.deleteEntityDefinition('00000000-0000-0000-0000-000000000000')
            ).rejects.toThrow()
        })
    })

    describe('applyEntityDefinitionSchema', () => {
        it('should apply schema for all entity definitions when called without uuid', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'Schema applied',
                data: { message: 'Schema applied to all entity definitions' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.applyEntityDefinitionSchema()

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/entity-definitions/apply-schema'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ uuid: undefined }),
                })
            )
        })

        it('should apply schema for a specific entity definition when called with uuid', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'Schema applied',
                data: { message: 'Schema applied to entity definition' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.applyEntityDefinitionSchema(
                'a1b2c3d4-e5f6-7890-abcd-ef1234567890'
            )

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/entity-definitions/apply-schema'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ uuid: 'a1b2c3d4-e5f6-7890-abcd-ef1234567890' }),
                })
            )
        })
    })

    describe('getEntityFields', () => {
        it('should return an array of field specs for a given entity type', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [
                    { name: 'title', type: 'String', required: true, system: false },
                    { name: 'uuid', type: 'Uuid', required: true, system: true },
                ],
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getEntityFields('product')

            expect(result).toBeDefined()
            expect(Array.isArray(result)).toBe(true)
            expect(result[0].name).toBe('title')
            expect(result[0].type).toBe('String')
            expect(result[0].required).toBe(true)
            expect(result[0].system).toBe(false)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/entity-definitions/product/fields'),
                expect.any(Object)
            )
        })
    })

    describe('listEntityDefinitionVersions', () => {
        it('should return a list of versions for a given entity definition uuid', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [
                    {
                        version_number: 2,
                        created_at: '2024-06-01T08:30:00Z',
                        created_by: 'c3d4e5f6-a7b8-9012-cdef-123456789012',
                        created_by_name: 'Alice',
                    },
                    {
                        version_number: 1,
                        created_at: '2024-01-15T10:00:00Z',
                        created_by: 'c3d4e5f6-a7b8-9012-cdef-123456789012',
                        created_by_name: 'Alice',
                    },
                ],
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.listEntityDefinitionVersions(
                'a1b2c3d4-e5f6-7890-abcd-ef1234567890'
            )

            expect(result).toBeDefined()
            expect(Array.isArray(result)).toBe(true)
            expect(result).toHaveLength(2)
            expect(result[0].version_number).toBe(2)
            expect(result[1].version_number).toBe(1)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/entity-definitions/a1b2c3d4-e5f6-7890-abcd-ef1234567890/versions'
                ),
                expect.any(Object)
            )
        })
    })

    describe('getEntityDefinitionVersion', () => {
        it('should return a specific version by uuid and version number', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: {
                    version_number: 1,
                    created_at: '2024-01-15T10:00:00Z',
                    created_by: 'c3d4e5f6-a7b8-9012-cdef-123456789012',
                    data: {
                        entity_type: 'product',
                        display_name: 'Product',
                        fields: [],
                    },
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getEntityDefinitionVersion(
                'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
                1
            )

            expect(result).toBeDefined()
            expect(result.version_number).toBe(1)
            expect(result.created_at).toBe('2024-01-15T10:00:00Z')
            expect(result.data).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/entity-definitions/a1b2c3d4-e5f6-7890-abcd-ef1234567890/versions/1'
                ),
                expect.any(Object)
            )
        })

        it('should throw on 404 when version does not exist', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                json: async () => ({ status: 'Error', message: 'Version not found' }),
            })

            await expect(
                client.getEntityDefinitionVersion(
                    'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
                    999
                )
            ).rejects.toThrow()
        })
    })
})
