import { describe, it, expect, vi, beforeEach } from 'vitest'
import { UsersClient } from './users'
import type { CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

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

const mockUser = {
    uuid: '01234567-89ab-7def-8123-456789abcdef',
    username: 'testuser',
    email: 'testuser@example.com',
    full_name: 'Test User',
    first_name: 'Test',
    last_name: 'User',
    role_uuids: ['01234567-89ab-7def-8123-456789abcde1'],
    status: 'Active',
    is_active: true,
    is_admin: false,
    super_admin: false,
    last_login: '2024-01-15T10:30:00Z',
    failed_login_attempts: 0,
    created_at: '2024-01-15T10:30:00Z',
    updated_at: '2024-01-15T10:30:00Z',
    created_by: '01234567-89ab-7def-8123-456789abcde0',
}

describe('UsersClient', () => {
    let client: UsersClient

    beforeEach(() => {
        client = new UsersClient()
        vi.clearAllMocks()
    })

    describe('getUsers', () => {
        it('should get users with pagination', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [mockUser],
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

            const result = await client.getUsers(1, 10)

            expect(result.data).toBeDefined()
            expect(Array.isArray(result.data)).toBe(true)
            expect(result.data).toHaveLength(1)
            expect(result.meta).toBeDefined()
            expect(result.meta?.pagination).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/users?page=1&per_page=10'),
                expect.any(Object)
            )
        })

        it('should get users with sorting', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [mockUser],
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

            const result = await client.getUsers(1, 10, 'username', 'asc')

            expect(result.data).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/users?page=1&per_page=10&sort_by=username&sort_order=asc'
                ),
                expect.any(Object)
            )
        })

        it('should get users with descending sort', async () => {
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

            const result = await client.getUsers(1, 10, 'created_at', 'desc')

            expect(result.data).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/users?page=1&per_page=10&sort_by=created_at&sort_order=desc'
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

            const result = await client.getUsers(1, 10, null, null)

            expect(result.data).toBeDefined()
            const url = mockFetch.mock.calls[0][0] as string
            expect(url).not.toContain('sort_by')
            expect(url).not.toContain('sort_order')
        })
    })

    describe('getUser', () => {
        it('should get a single user successfully', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: mockUser,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getUser('01234567-89ab-7def-8123-456789abcdef')

            expect(result).toBeDefined()
            expect(result.uuid).toBe('01234567-89ab-7def-8123-456789abcdef')
            expect(result.username).toBe('testuser')
            expect(result.email).toBe('testuser@example.com')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/users/01234567-89ab-7def-8123-456789abcdef'),
                expect.any(Object)
            )
        })

        it('should handle 404 error when user not found', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => ({ status: 'Error', message: 'User not found' }),
            })

            await expect(
                client.getUser('01234567-89ab-7def-8123-000000000000')
            ).rejects.toThrow()
        })
    })

    describe('createUser', () => {
        it('should create a user successfully', async () => {
            const request: CreateUserRequest = {
                username: 'newuser',
                email: 'newuser@example.com',
                password: 'securepassword123',
                first_name: 'New',
                last_name: 'User',
            }

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: {
                    ...mockUser,
                    uuid: '01234567-89ab-7def-8123-456789abcde2',
                    username: 'newuser',
                    email: 'newuser@example.com',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.createUser(request)

            expect(result).toBeDefined()
            expect(result.username).toBe('newuser')
            expect(result.email).toBe('newuser@example.com')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/users'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify(request),
                })
            )
        })

        it('should handle 422 validation error on create', async () => {
            const request: CreateUserRequest = {
                username: '',
                email: 'not-an-email',
                password: '123',
                first_name: 'New',
                last_name: 'User',
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 422,
                statusText: 'Unprocessable Entity',
                json: async () => ({ status: 'Error', message: 'Validation failed' }),
            })

            await expect(client.createUser(request)).rejects.toThrow()
        })
    })

    describe('updateUser', () => {
        it('should update a user successfully', async () => {
            const request: UpdateUserRequest = {
                email: 'updated@example.com',
                first_name: 'Updated',
            }

            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: {
                    ...mockUser,
                    email: 'updated@example.com',
                    first_name: 'Updated',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.updateUser(
                '01234567-89ab-7def-8123-456789abcdef',
                request
            )

            expect(result).toBeDefined()
            expect(result.email).toBe('updated@example.com')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/users/01234567-89ab-7def-8123-456789abcdef'),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(request),
                })
            )
        })

        it('should handle 404 error when updating non-existent user', async () => {
            const request: UpdateUserRequest = {
                email: 'updated@example.com',
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => ({ status: 'Error', message: 'User not found' }),
            })

            await expect(
                client.updateUser('01234567-89ab-7def-8123-000000000000', request)
            ).rejects.toThrow()
        })
    })

    describe('deleteUser', () => {
        it('should delete a user successfully', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'User deleted',
                data: { message: 'User deleted' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.deleteUser('01234567-89ab-7def-8123-456789abcdef')

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/users/01234567-89ab-7def-8123-456789abcdef'),
                expect.objectContaining({
                    method: 'DELETE',
                })
            )
        })

        it('should handle 404 error when deleting non-existent user', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => ({ status: 'Error', message: 'User not found' }),
            })

            await expect(
                client.deleteUser('01234567-89ab-7def-8123-000000000000')
            ).rejects.toThrow()
        })
    })

    describe('getUserRoles', () => {
        it('should get user roles successfully', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'OK',
                data: [
                    '01234567-89ab-7def-8123-456789abcde1',
                    '01234567-89ab-7def-8123-456789abcde2',
                ],
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.getUserRoles('01234567-89ab-7def-8123-456789abcdef')

            expect(result).toBeDefined()
            expect(Array.isArray(result)).toBe(true)
            expect(result).toHaveLength(2)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/users/01234567-89ab-7def-8123-456789abcdef/roles'
                ),
                expect.any(Object)
            )
        })

        it('should handle 404 error when getting roles for non-existent user', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => ({ status: 'Error', message: 'User not found' }),
            })

            await expect(
                client.getUserRoles('01234567-89ab-7def-8123-000000000000')
            ).rejects.toThrow()
        })
    })

    describe('assignRolesToUser', () => {
        it('should assign roles to a user successfully', async () => {
            const roleUuids = [
                '01234567-89ab-7def-8123-456789abcde1',
                '01234567-89ab-7def-8123-456789abcde2',
            ]

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
                '01234567-89ab-7def-8123-456789abcdef',
                roleUuids
            )

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/users/01234567-89ab-7def-8123-456789abcdef/roles'
                ),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(roleUuids),
                })
            )
        })

        it('should assign empty roles array to a user', async () => {
            const mockResponse = {
                status: 'Success',
                message: 'Roles cleared',
                data: { message: 'Roles cleared' },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockResponse,
            })

            const result = await client.assignRolesToUser(
                '01234567-89ab-7def-8123-456789abcdef',
                []
            )

            expect(result).toBeDefined()
            expect(result.message).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(
                    '/admin/api/v1/users/01234567-89ab-7def-8123-456789abcdef/roles'
                ),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify([]),
                })
            )
        })
    })
})
