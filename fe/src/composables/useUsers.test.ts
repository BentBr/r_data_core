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
        getUserSchemes: vi.fn(),
        assignSchemesToUser: vi.fn(),
    },
}))

vi.mock('@/composables/useSnackbar', () => ({
    useSnackbar: () => ({
        showSuccess: vi.fn(),
        showError: vi.fn(),
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
                role: 'Editor',
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

        expect(typedHttpClient.getUsers).toHaveBeenCalledWith(1, 20)
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
            role: 'Editor',
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
            role: 'Editor',
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
})
