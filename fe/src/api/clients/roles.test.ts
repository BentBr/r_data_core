import { describe, it, expect, vi, beforeEach } from 'vitest'
import { RolesClient } from './roles'
import type { CreateRoleRequest, UpdateRoleRequest, AssignRolesRequest } from '@/types/schemas'

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

const mockPermission = {
    resource_type: 'Entity',
    permission_type: 'Read' as const,
    access_level: 'All' as const,
    resource_uuids: [],
    constraints: null,
}

const mockRole = {
    uuid: '01234567-89ab-7def-8123-456789abcdef',
    name: 'Admin',
    description: 'Administrator role',
    is_system: false,
    super_admin: false,
    permissions: [mockPermission],
    created_at: '2024-01-15T10:30:00Z',
    updated_at: '2024-01-15T10:30:00Z',
    created_by: '01234567-89ab-7def-8123-456789abcde0',
    updated_by: null,
    published: true,
    version: 1,
}

describe('RolesClient', () => {
    let client: RolesClient

    beforeEach(() => {
        client = new RolesClient()
        vi.clearAllMocks()
    })

    describe('getRoles', () => {
        it('should get roles with pagination', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [mockRole],
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

            const result = await client.getRoles(1, 10)

            expect(result.data).toBeDefined()
            expect(Array.isArray(result.data)).toBe(true)
            expect(result.data).toHaveLength(1)
            expect(result.meta).toBeDefined()
            expect(result.meta?.pagination).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/roles?page=1&per_page=10'),
                expect.any(Object)
            )
        })

        it('should get roles with sorting', async () => {
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

            const result = await client.getRoles(1, 10, 'name', 'asc')

            expect(result.data).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/roles?page=1&per_page=10&sort_by=name&sort_order=asc'
                ),
                expect.any(Object)
            )
        })

        it('should get roles with descending sort', async () => {
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

            const result = await client.getRoles(1, 10, 'created_at', 'desc')

            expect(result.data).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/roles?page=1&per_page=10&sort_by=created_at&sort_order=desc'
                ),
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

            const result = await client.getRoles(1, 10, null, null)

            expect(result.data).toBeDefined()
            const url = mockFetch.mock.calls[0][0] as string
            expect(url).not.toContain('sort_by')
            expect(url).not.toContain('sort_order')
        })
    })

    describe('getRole', () => {
        it('should get a single role successfully', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: mockRole,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getRole('01234567-89ab-7def-8123-456789abcdef')

            expect(result).toBeDefined()
            expect(result.uuid).toBe('01234567-89ab-7def-8123-456789abcdef')
            expect(result.name).toBe('Admin')
            expect(result.permissions).toHaveLength(1)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/roles/01234567-89ab-7def-8123-456789abcdef'),
                expect.any(Object)
            )
        })

        it('should handle 404 error when role not found', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => ({ status: 'Error', message: 'Role not found' }),
            })

            await expect(client.getRole('01234567-89ab-7def-8123-000000000000')).rejects.toThrow()
        })
    })

    describe('createRole', () => {
        it('should create a role successfully', async () => {
            const request: CreateRoleRequest = {
                name: 'Editor',
                description: 'Can edit content',
                permissions: [mockPermission],
            }

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: {
                    ...mockRole,
                    uuid: '01234567-89ab-7def-8123-456789abcde3',
                    name: 'Editor',
                    description: 'Can edit content',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.createRole(request)

            expect(result).toBeDefined()
            expect(result.name).toBe('Editor')
            expect(result.description).toBe('Can edit content')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/roles'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify(request),
                })
            )
        })

        it('should handle 422 validation error on create', async () => {
            const request: CreateRoleRequest = {
                name: '',
                description: null,
                permissions: [],
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 422,
                statusText: 'Unprocessable Entity',
                json: async () => ({ status: 'Error', message: 'Validation failed' }),
            })

            await expect(client.createRole(request)).rejects.toThrow()
        })
    })

    describe('updateRole', () => {
        it('should update a role successfully', async () => {
            const request: UpdateRoleRequest = {
                name: 'Senior Editor',
                description: 'Can edit and publish content',
                permissions: [mockPermission],
            }

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: {
                    ...mockRole,
                    name: 'Senior Editor',
                    description: 'Can edit and publish content',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.updateRole('01234567-89ab-7def-8123-456789abcdef', request)

            expect(result).toBeDefined()
            expect(result.name).toBe('Senior Editor')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/roles/01234567-89ab-7def-8123-456789abcdef'),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(request),
                })
            )
        })

        it('should handle 404 error when updating non-existent role', async () => {
            const request: UpdateRoleRequest = {
                name: 'Updated',
                description: null,
                permissions: [],
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => ({ status: 'Error', message: 'Role not found' }),
            })

            await expect(
                client.updateRole('01234567-89ab-7def-8123-000000000000', request)
            ).rejects.toThrow()
        })
    })

    describe('deleteRole', () => {
        it('should delete a role successfully', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'Role deleted',
                data: { message: 'Role deleted' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.deleteRole('01234567-89ab-7def-8123-456789abcdef')

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/roles/01234567-89ab-7def-8123-456789abcdef'),
                expect.objectContaining({
                    method: 'DELETE',
                })
            )
        })

        it('should handle 404 error when deleting non-existent role', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => ({ status: 'Error', message: 'Role not found' }),
            })

            await expect(
                client.deleteRole('01234567-89ab-7def-8123-000000000000')
            ).rejects.toThrow()
        })
    })

    describe('assignRolesToUser', () => {
        it('should assign roles to a user successfully', async () => {
            const request: AssignRolesRequest = {
                role_uuids: [
                    '01234567-89ab-7def-8123-456789abcdef',
                    '01234567-89ab-7def-8123-456789abcde1',
                ],
            }

            const mockResponse = {
                status: 'Success',
                message: 'Roles assigned',
                data: { message: 'Roles assigned' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.assignRolesToUser(
                '01234567-89ab-7def-8123-456789abcde2',
                request
            )

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/roles/users/01234567-89ab-7def-8123-456789abcde2/roles'
                ),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(request),
                })
            )
        })

        it('should handle 422 validation error when assigning roles to user', async () => {
            const request: AssignRolesRequest = {
                role_uuids: ['invalid-uuid'],
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 422,
                statusText: 'Unprocessable Entity',
                json: async () => ({ status: 'Error', message: 'Validation failed' }),
            })

            await expect(
                client.assignRolesToUser('01234567-89ab-7def-8123-456789abcde2', request)
            ).rejects.toThrow()
        })
    })

    describe('assignRolesToApiKey', () => {
        it('should assign roles to an API key successfully', async () => {
            const request: AssignRolesRequest = {
                role_uuids: ['01234567-89ab-7def-8123-456789abcdef'],
            }

            const mockResponse = {
                status: 'Success',
                message: 'Roles assigned to API key',
                data: { message: 'Roles assigned to API key' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.assignRolesToApiKey(
                '01234567-89ab-7def-8123-456789abcde3',
                request
            )

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/roles/api-keys/01234567-89ab-7def-8123-456789abcde3/roles'
                ),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(request),
                })
            )
        })

        it('should handle 404 error when assigning roles to non-existent API key', async () => {
            const request: AssignRolesRequest = {
                role_uuids: ['01234567-89ab-7def-8123-456789abcdef'],
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => ({ status: 'Error', message: 'API key not found' }),
            })

            await expect(
                client.assignRolesToApiKey('01234567-89ab-7def-8123-000000000000', request)
            ).rejects.toThrow()
        })
    })
})
