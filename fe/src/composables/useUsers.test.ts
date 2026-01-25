import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useUsers } from './useUsers'
import { typedHttpClient } from '@/api/typed-client'
import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getUsers: vi.fn(),
        getUser: vi.fn(),
        createUser: vi.fn(),
        updateUser: vi.fn(),
        deleteUser: vi.fn(),
        getUserRoles: vi.fn(),
        assignRolesToUser: vi.fn(),
    },
}))

const mockHandleError = vi.fn()
const mockHandleSuccess = vi.fn()
const mockT = vi.fn((key: string) => key)

vi.mock('@/composables/useErrorHandler', () => ({
    useErrorHandler: () => ({
        handleError: mockHandleError,
        handleSuccess: mockHandleSuccess,
    }),
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: mockT,
    }),
}))

describe('useUsers', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('should load users successfully', async () => {
        const mockUsers: UserResponse[] = [
            {
                uuid: '123e4567-e89b-12d3-a456-426614174000',
                username: 'testuser',
                email: 'test@example.com',
                full_name: 'Test User',
                first_name: 'Test',
                last_name: 'User',
                role_uuids: ['123e4567-e89b-12d3-a456-426614174002'],
                status: 'Active',
                is_active: true,
                is_admin: false,
                super_admin: false,
                last_login: null,
                failed_login_attempts: 0,
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-01-01T00:00:00Z',
                created_by: '123e4567-e89b-12d3-a456-426614174001',
            },
        ]

        vi.mocked(typedHttpClient.getUsers).mockResolvedValue({
            data: mockUsers,
            meta: {
                pagination: {
                    total: 1,
                    page: 1,
                    per_page: 20,
                    total_pages: 1,
                    has_previous: false,
                    has_next: false,
                },
            },
        })

        const { loadUsers, users } = useUsers()
        await loadUsers()

        expect(typedHttpClient.getUsers).toHaveBeenCalledWith(1, 20, undefined, undefined)
        expect(users.value).toEqual(mockUsers)
    })

    it('should create a user successfully', async () => {
        const createData: CreateUserRequest = {
            username: 'newuser',
            email: 'new@example.com',
            password: 'password123',
            first_name: 'New',
            last_name: 'User',
            is_active: true,
            super_admin: false,
        }

        const mockUser: UserResponse = {
            uuid: '123e4567-e89b-12d3-a456-426614174000',
            username: 'newuser',
            email: 'new@example.com',
            full_name: 'New User',
            first_name: 'New',
            last_name: 'User',
            role_uuids: ['123e4567-e89b-12d3-a456-426614174002'],
            status: 'Active',
            is_active: true,
            is_admin: false,
            super_admin: false,
            last_login: null,
            failed_login_attempts: 0,
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:00:00Z',
            created_by: '123e4567-e89b-12d3-a456-426614174001',
        }

        vi.mocked(typedHttpClient.createUser).mockResolvedValue(mockUser)

        const { createUser } = useUsers()
        await createUser(createData)

        expect(typedHttpClient.createUser).toHaveBeenCalledWith(createData)
    })

    it('should update a user successfully', async () => {
        const updateData: UpdateUserRequest = {
            email: 'updated@example.com',
            first_name: 'Updated',
            last_name: 'Name',
            super_admin: true,
        }

        const mockUser: UserResponse = {
            uuid: '123e4567-e89b-12d3-a456-426614174000',
            username: 'testuser',
            email: 'updated@example.com',
            full_name: 'Updated Name',
            first_name: 'Updated',
            last_name: 'Name',
            role_uuids: ['123e4567-e89b-12d3-a456-426614174002'],
            status: 'Active',
            is_active: true,
            is_admin: false,
            super_admin: true,
            last_login: null,
            failed_login_attempts: 0,
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:00:00Z',
            created_by: '123e4567-e89b-12d3-a456-426614174001',
        }

        vi.mocked(typedHttpClient.updateUser).mockResolvedValue(mockUser)

        const { updateUser } = useUsers()
        await updateUser('123e4567-e89b-12d3-a456-426614174000', updateData)

        expect(typedHttpClient.updateUser).toHaveBeenCalledWith(
            '123e4567-e89b-12d3-a456-426614174000',
            updateData
        )
    })

    it('should delete a user successfully', async () => {
        vi.mocked(typedHttpClient.deleteUser).mockResolvedValue({ message: 'User deleted' })

        const { deleteUser } = useUsers()
        await deleteUser('123e4567-e89b-12d3-a456-426614174000')

        expect(typedHttpClient.deleteUser).toHaveBeenCalledWith(
            '123e4567-e89b-12d3-a456-426614174000'
        )
    })

    describe('error handling', () => {
        it('should handle 403 error when creating user', async () => {
            const createData: CreateUserRequest = {
                username: 'newuser',
                email: 'new@example.com',
                password: 'password123',
                first_name: 'New',
                last_name: 'User',
                is_active: true,
                super_admin: false,
            }

            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.createUser).mockRejectedValue(error)

            const { createUser } = useUsers()

            await expect(createUser(createData)).rejects.toThrow('HTTP 403: Forbidden')
            expect(mockHandleError).toHaveBeenCalledWith(error)
        })

        it('should handle 403 error when updating user', async () => {
            const updateData: UpdateUserRequest = {
                email: 'updated@example.com',
                first_name: 'Updated',
                last_name: 'Name',
            }

            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.updateUser).mockRejectedValue(error)

            const { updateUser } = useUsers()

            await expect(
                updateUser('123e4567-e89b-12d3-a456-426614174000', updateData)
            ).rejects.toThrow('HTTP 403: Forbidden')
            expect(mockHandleError).toHaveBeenCalledWith(error)
        })

        it('should handle 403 error when deleting user', async () => {
            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.deleteUser).mockRejectedValue(error)

            const { deleteUser } = useUsers()

            await expect(deleteUser('123e4567-e89b-12d3-a456-426614174000')).rejects.toThrow(
                'HTTP 403: Forbidden'
            )
            expect(mockHandleError).toHaveBeenCalledWith(error)
        })

        it('should handle 403 error when loading users', async () => {
            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.getUsers).mockRejectedValue(error)

            const { loadUsers, error: errorRef } = useUsers()

            await expect(loadUsers()).rejects.toThrow('HTTP 403: Forbidden')
            expect(mockHandleError).toHaveBeenCalledWith(error)
            expect(errorRef.value).toBe('HTTP 403: Forbidden')
        })

        it('should handle 403 error when getting user', async () => {
            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.getUser).mockRejectedValue(error)

            const { getUser, error: errorRef } = useUsers()

            await expect(getUser('123e4567-e89b-12d3-a456-426614174000')).rejects.toThrow(
                'HTTP 403: Forbidden'
            )
            expect(mockHandleError).toHaveBeenCalledWith(error)
            expect(errorRef.value).toBe('HTTP 403: Forbidden')
        })

        it('should handle 403 error when getting user roles', async () => {
            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.getUserRoles).mockRejectedValue(error)

            const { getUserRoles } = useUsers()

            await expect(getUserRoles('123e4567-e89b-12d3-a456-426614174000')).rejects.toThrow(
                'HTTP 403: Forbidden'
            )
            expect(mockHandleError).toHaveBeenCalledWith(error)
        })

        it('should handle 403 error when assigning roles to user', async () => {
            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.assignRolesToUser).mockRejectedValue(error)

            const { assignRolesToUser } = useUsers()

            await expect(
                assignRolesToUser('123e4567-e89b-12d3-a456-426614174000', [
                    '123e4567-e89b-12d3-a456-426614174002',
                ])
            ).rejects.toThrow('HTTP 403: Forbidden')
            expect(mockHandleError).toHaveBeenCalledWith(error)
        })
    })

    describe('success messages', () => {
        it('should show success message when creating user', async () => {
            const createData: CreateUserRequest = {
                username: 'newuser',
                email: 'new@example.com',
                password: 'password123',
                first_name: 'New',
                last_name: 'User',
                is_active: true,
                super_admin: false,
            }

            const mockUser: UserResponse = {
                uuid: '123e4567-e89b-12d3-a456-426614174000',
                username: 'newuser',
                email: 'new@example.com',
                full_name: 'New User',
                first_name: 'New',
                last_name: 'User',
                role_uuids: [],
                status: 'Active',
                is_active: true,
                is_admin: false,
                super_admin: false,
                last_login: null,
                failed_login_attempts: 0,
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-01-01T00:00:00Z',
                created_by: '123e4567-e89b-12d3-a456-426614174001',
            }

            vi.mocked(typedHttpClient.createUser).mockResolvedValue(mockUser)

            const { createUser } = useUsers()
            await createUser(createData)

            expect(mockHandleSuccess).toHaveBeenCalledWith('users.create.success')
        })

        it('should show success message when updating user', async () => {
            const updateData: UpdateUserRequest = {
                email: 'updated@example.com',
            }

            const mockUser: UserResponse = {
                uuid: '123e4567-e89b-12d3-a456-426614174000',
                username: 'testuser',
                email: 'updated@example.com',
                full_name: 'Test User',
                first_name: 'Test',
                last_name: 'User',
                role_uuids: [],
                status: 'Active',
                is_active: true,
                is_admin: false,
                super_admin: false,
                last_login: null,
                failed_login_attempts: 0,
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-01-01T00:00:00Z',
                created_by: '123e4567-e89b-12d3-a456-426614174001',
            }

            vi.mocked(typedHttpClient.updateUser).mockResolvedValue(mockUser)

            const { updateUser } = useUsers()
            await updateUser('123e4567-e89b-12d3-a456-426614174000', updateData)

            expect(mockHandleSuccess).toHaveBeenCalledWith('users.update.success')
        })

        it('should show success message when deleting user', async () => {
            vi.mocked(typedHttpClient.deleteUser).mockResolvedValue({ message: 'User deleted' })

            const { deleteUser } = useUsers()
            await deleteUser('123e4567-e89b-12d3-a456-426614174000')

            expect(mockHandleSuccess).toHaveBeenCalledWith('users.delete.success')
        })

        it('should show success message when assigning roles', async () => {
            vi.mocked(typedHttpClient.assignRolesToUser).mockResolvedValue({
                message: 'Roles assigned',
            })

            const { assignRolesToUser } = useUsers()
            await assignRolesToUser('123e4567-e89b-12d3-a456-426614174000', [
                '123e4567-e89b-12d3-a456-426614174002',
            ])

            expect(mockHandleSuccess).toHaveBeenCalledWith('users.roles.assign.success')
        })
    })
})
