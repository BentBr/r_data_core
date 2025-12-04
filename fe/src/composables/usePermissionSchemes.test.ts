import { describe, it, expect, vi, beforeEach } from 'vitest'
import { usePermissionSchemes } from './usePermissionSchemes'
import type {
    PermissionScheme,
    CreatePermissionSchemeRequest,
    UpdatePermissionSchemeRequest,
} from '@/types/schemas'

const mockGetPermissionSchemes = vi.fn()
const mockGetPermissionScheme = vi.fn()
const mockCreatePermissionScheme = vi.fn()
const mockUpdatePermissionScheme = vi.fn()
const mockDeletePermissionScheme = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getPermissionSchemes: (page?: number, itemsPerPage?: number) =>
            mockGetPermissionSchemes(page, itemsPerPage),
        getPermissionScheme: (uuid: string) => mockGetPermissionScheme(uuid),
        createPermissionScheme: (data: CreatePermissionSchemeRequest) =>
            mockCreatePermissionScheme(data),
        updatePermissionScheme: (uuid: string, data: UpdatePermissionSchemeRequest) =>
            mockUpdatePermissionScheme(uuid, data),
        deletePermissionScheme: (uuid: string) => mockDeletePermissionScheme(uuid),
    },
}))

const showSuccess = vi.fn()
const showError = vi.fn()
vi.mock('@/composables/useSnackbar', () => ({
    useSnackbar: () => ({
        showSuccess,
        showError,
    }),
}))

describe('usePermissionSchemes', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    describe('loadSchemes', () => {
        it('should show error message for 404 errors', async () => {
            const error = new Error('API resource not found not found')
            mockGetPermissionSchemes.mockRejectedValue(error)

            const { loadSchemes } = usePermissionSchemes()

            await expect(loadSchemes()).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('API resource not found not found')
        })

        it('should show error message for 500 errors', async () => {
            const error = new Error('Internal server error')
            mockGetPermissionSchemes.mockRejectedValue(error)

            const { loadSchemes } = usePermissionSchemes()

            await expect(loadSchemes()).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('Internal server error')
        })

        it('should show error message for 400 errors', async () => {
            const error = new Error('Bad request')
            mockGetPermissionSchemes.mockRejectedValue(error)

            const { loadSchemes } = usePermissionSchemes()

            await expect(loadSchemes()).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('Bad request')
        })

        it('should show error message for network errors', async () => {
            const error = new Error('Network error')
            mockGetPermissionSchemes.mockRejectedValue(error)

            const { loadSchemes } = usePermissionSchemes()

            await expect(loadSchemes()).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('Network error')
        })
    })

    describe('createScheme', () => {
        it('should show error message for 404 errors', async () => {
            const error = new Error('API resource not found not found')
            mockCreatePermissionScheme.mockRejectedValue(error)

            const { createScheme } = usePermissionSchemes()

            await expect(
                createScheme({
                    name: 'Test Scheme',
                    description: 'Test',
                    role_permissions: {},
                })
            ).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('API resource not found not found')
        })

        it('should show error message for 500 errors', async () => {
            const error = new Error('Internal server error')
            mockCreatePermissionScheme.mockRejectedValue(error)

            const { createScheme } = usePermissionSchemes()

            await expect(
                createScheme({
                    name: 'Test Scheme',
                    description: 'Test',
                    role_permissions: {},
                })
            ).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('Internal server error')
        })
    })

    describe('updateScheme', () => {
        it('should show error message for 404 errors', async () => {
            const error = new Error('Permission scheme not found')
            mockUpdatePermissionScheme.mockRejectedValue(error)

            const { updateScheme } = usePermissionSchemes()

            await expect(
                updateScheme('test-uuid', {
                    name: 'Test Scheme',
                    description: 'Test',
                    role_permissions: {},
                })
            ).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('Permission scheme not found')
        })

        it('should show error message for 500 errors', async () => {
            const error = new Error('Internal server error')
            mockUpdatePermissionScheme.mockRejectedValue(error)

            const { updateScheme } = usePermissionSchemes()

            await expect(
                updateScheme('test-uuid', {
                    name: 'Test Scheme',
                    description: 'Test',
                    role_permissions: {},
                })
            ).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('Internal server error')
        })
    })

    describe('deleteScheme', () => {
        it('should show error message for 404 errors', async () => {
            const error = new Error('Permission scheme not found')
            mockDeletePermissionScheme.mockRejectedValue(error)

            const { deleteScheme } = usePermissionSchemes()

            await expect(deleteScheme('test-uuid')).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('Permission scheme not found')
        })

        it('should show error message for 500 errors', async () => {
            const error = new Error('Internal server error')
            mockDeletePermissionScheme.mockRejectedValue(error)

            const { deleteScheme } = usePermissionSchemes()

            await expect(deleteScheme('test-uuid')).rejects.toThrow()

            expect(showError).toHaveBeenCalledWith('Internal server error')
        })
    })
})

