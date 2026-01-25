import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useRoles } from './useRoles'
import { typedHttpClient } from '@/api/typed-client'
import type { Role, CreateRoleRequest, UpdateRoleRequest } from '@/types/schemas'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getRoles: vi.fn(),
        createRole: vi.fn(),
        updateRole: vi.fn(),
        deleteRole: vi.fn(),
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

describe('useRoles', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    describe('success scenarios', () => {
        it('should load roles successfully', async () => {
            const mockRoles: Role[] = [
                {
                    uuid: '123e4567-e89b-12d3-a456-426614174000',
                    name: 'Test Role',
                    description: 'Test description',
                    created_at: '2024-01-01T00:00:00Z',
                    updated_at: '2024-01-01T00:00:00Z',
                    created_by: '123e4567-e89b-12d3-a456-426614174001',
                    updated_by: null,
                    permissions: [],
                    super_admin: false,
                    is_system: false,
                    published: false,
                    version: 1,
                },
            ]

            vi.mocked(typedHttpClient.getRoles).mockResolvedValue({
                data: mockRoles,
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

            const { loadRoles, roles } = useRoles()
            await loadRoles()

            expect(typedHttpClient.getRoles).toHaveBeenCalledWith(1, 20, undefined, undefined)
            expect(roles.value).toEqual(mockRoles)
        })

        it('should create a role successfully', async () => {
            const createData: CreateRoleRequest = {
                name: 'New Role',
                description: 'New role description',
                permissions: [],
                super_admin: false,
            }

            const mockRole: Role = {
                uuid: '123e4567-e89b-12d3-a456-426614174000',
                name: 'New Role',
                description: 'New role description',
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-01-01T00:00:00Z',
                created_by: '123e4567-e89b-12d3-a456-426614174001',
                updated_by: null,
                permissions: [],
                super_admin: false,
                is_system: false,
                published: false,
                version: 1,
            }

            vi.mocked(typedHttpClient.createRole).mockResolvedValue(mockRole)

            const { createRole } = useRoles()
            await createRole(createData)

            expect(typedHttpClient.createRole).toHaveBeenCalledWith(createData)
            expect(mockHandleSuccess).toHaveBeenCalledWith('roles.create.success')
        })

        it('should update a role successfully', async () => {
            const updateData: UpdateRoleRequest = {
                name: 'Updated Role',
                description: 'Updated description',
                permissions: [],
            }

            const mockRole: Role = {
                uuid: '123e4567-e89b-12d3-a456-426614174000',
                name: 'Updated Role',
                description: 'Updated description',
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-01-01T00:00:00Z',
                created_by: '123e4567-e89b-12d3-a456-426614174001',
                updated_by: null,
                permissions: [],
                super_admin: false,
                is_system: false,
                published: false,
                version: 1,
            }

            vi.mocked(typedHttpClient.updateRole).mockResolvedValue(mockRole)

            const { updateRole } = useRoles()
            await updateRole('123e4567-e89b-12d3-a456-426614174000', updateData)

            expect(typedHttpClient.updateRole).toHaveBeenCalledWith(
                '123e4567-e89b-12d3-a456-426614174000',
                updateData
            )
            expect(mockHandleSuccess).toHaveBeenCalledWith('roles.update.success')
        })

        it('should delete a role successfully', async () => {
            vi.mocked(typedHttpClient.deleteRole).mockResolvedValue({ message: 'Role deleted' })

            const { deleteRole } = useRoles()
            await deleteRole('123e4567-e89b-12d3-a456-426614174000')

            expect(typedHttpClient.deleteRole).toHaveBeenCalledWith(
                '123e4567-e89b-12d3-a456-426614174000'
            )
            expect(mockHandleSuccess).toHaveBeenCalledWith('roles.delete.success')
        })
    })

    describe('error handling', () => {
        it('should handle 403 error when creating role', async () => {
            const createData: CreateRoleRequest = {
                name: 'New Role',
                description: 'New role description',
                permissions: [],
                super_admin: false,
            }

            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.createRole).mockRejectedValue(error)

            const { createRole } = useRoles()

            await expect(createRole(createData)).rejects.toThrow('HTTP 403: Forbidden')
            expect(mockHandleError).toHaveBeenCalledWith(error)
        })

        it('should handle 403 error when updating role', async () => {
            const updateData: UpdateRoleRequest = {
                name: 'Updated Role',
                description: 'Updated description',
                permissions: [],
            }

            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.updateRole).mockRejectedValue(error)

            const { updateRole } = useRoles()

            await expect(
                updateRole('123e4567-e89b-12d3-a456-426614174000', updateData)
            ).rejects.toThrow('HTTP 403: Forbidden')
            expect(mockHandleError).toHaveBeenCalledWith(error)
        })

        it('should handle 403 error when deleting role', async () => {
            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.deleteRole).mockRejectedValue(error)

            const { deleteRole } = useRoles()

            await expect(deleteRole('123e4567-e89b-12d3-a456-426614174000')).rejects.toThrow(
                'HTTP 403: Forbidden'
            )
            expect(mockHandleError).toHaveBeenCalledWith(error)
        })

        it('should handle 403 error when loading roles', async () => {
            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.getRoles).mockRejectedValue(error)

            const { loadRoles, error: errorRef } = useRoles()

            await expect(loadRoles()).rejects.toThrow('HTTP 403: Forbidden')
            expect(mockHandleError).toHaveBeenCalledWith(error)
            expect(errorRef.value).toBe('HTTP 403: Forbidden')
        })

        it('should propagate errors after handling', async () => {
            const error = new Error('HTTP 403: Forbidden')
            vi.mocked(typedHttpClient.createRole).mockRejectedValue(error)

            const { createRole } = useRoles()

            await expect(
                createRole({
                    name: 'Test Role',
                    permissions: [],
                    super_admin: false,
                } as CreateRoleRequest)
            ).rejects.toThrow('HTTP 403: Forbidden')
            expect(mockHandleError).toHaveBeenCalled()
        })
    })
})
