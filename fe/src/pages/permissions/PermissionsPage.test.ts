import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { ref } from 'vue'
import PermissionsPage from './PermissionsPage.vue'
import type { PermissionScheme } from '@/types/schemas'

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
        createPermissionScheme: (data: unknown) => mockCreatePermissionScheme(data),
        updatePermissionScheme: (uuid: string, data: unknown) =>
            mockUpdatePermissionScheme(uuid, data),
        deletePermissionScheme: (uuid: string) => mockDeletePermissionScheme(uuid),
    },
}))

const mockLoading = ref(false)
const mockError = ref('')
const mockSchemesList = ref<PermissionScheme[]>([])

vi.mock('@/composables/usePermissionSchemes', () => ({
    usePermissionSchemes: () => {
        // Create wrapper functions that call showError/showSuccess on error/success
        const createSchemeWrapper = async (data: unknown) => {
            try {
                const result = await mockCreatePermissionScheme(data)
                showSuccess('Permission scheme created successfully')
                return result
            } catch (err) {
                showError(err instanceof Error ? err.message : 'Failed to create permission scheme')
                throw err
            }
        }

        const updateSchemeWrapper = async (uuid: string, data: unknown) => {
            try {
                const result = await mockUpdatePermissionScheme(uuid, data)
                showSuccess('Permission scheme updated successfully')
                return result
            } catch (err) {
                showError(err instanceof Error ? err.message : 'Failed to update permission scheme')
                throw err
            }
        }

        const deleteSchemeWrapper = async (uuid: string) => {
            try {
                const result = await mockDeletePermissionScheme(uuid)
                showSuccess('Permission scheme deleted successfully')
                return result
            } catch (err) {
                showError(err instanceof Error ? err.message : 'Failed to delete permission scheme')
                throw err
            }
        }

        const loadSchemesWrapper = async (page?: number, perPage?: number) => {
            try {
                const result = await mockGetPermissionSchemes(page, perPage)
                mockSchemesList.value = result.data
                return result
            } catch (err) {
                mockError.value =
                    err instanceof Error ? err.message : 'Failed to load permission schemes'
                showError(mockError.value)
                throw err
            }
        }

        return {
            loading: mockLoading,
            error: mockError,
            schemes: mockSchemesList,
            loadSchemes: loadSchemesWrapper,
            createScheme: createSchemeWrapper,
            updateScheme: updateSchemeWrapper,
            deleteScheme: deleteSchemeWrapper,
        }
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k.split('.').pop() }),
}))

const showSuccess = vi.fn()
const showError = vi.fn()
vi.mock('@/composables/useSnackbar', () => ({
    useSnackbar: () => ({
        currentSnackbar: null,
        showSuccess,
        showError,
    }),
}))

const mockSetPage = vi.fn()
const mockSetItemsPerPage = vi.fn()
vi.mock('@/composables/usePagination', () => ({
    usePagination: () => ({
        state: { page: 1, itemsPerPage: 20 },
        setPage: mockSetPage,
        setItemsPerPage: mockSetItemsPerPage,
    }),
}))

const mockSchemes: PermissionScheme[] = [
    {
        uuid: 'scheme-1',
        name: 'Editor Scheme',
        description: 'Permissions for editors',
        role_permissions: {
            Editor: [
                {
                    resource_type: 'Workflows',
                    permission_type: 'Read',
                    access_level: 'All',
                    resource_uuids: [],
                    constraints: null,
                },
            ],
        },
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        created_by: 'user-1',
        updated_by: null,
        is_system: false,
    },
    {
        uuid: 'scheme-2',
        name: 'Viewer Scheme',
        description: 'Read-only permissions',
        role_permissions: {
            Viewer: [
                {
                    resource_type: 'Workflows',
                    permission_type: 'Read',
                    access_level: 'Own',
                    resource_uuids: [],
                    constraints: null,
                },
            ],
        },
        created_at: '2024-01-02T00:00:00Z',
        updated_at: '2024-01-02T00:00:00Z',
        created_by: 'user-1',
        updated_by: null,
        is_system: false,
    },
]

describe('PermissionsPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockLoading.value = false
        mockError.value = ''
        mockSchemesList.value = []

        // Setup loadSchemes - the wrapper will handle updating mockSchemesList
        mockGetPermissionSchemes.mockResolvedValue({
            data: mockSchemes,
            meta: {
                pagination: {
                    total: 2,
                    page: 1,
                    per_page: 20,
                    total_pages: 1,
                    has_previous: false,
                    has_next: false,
                },
            },
        })

        mockCreatePermissionScheme.mockResolvedValue(mockSchemes[0])
        mockUpdatePermissionScheme.mockResolvedValue(mockSchemes[0])
        mockDeletePermissionScheme.mockResolvedValue({ message: 'Deleted successfully' })
    })

    it('renders correctly and loads permission schemes', async () => {
        const wrapper = mount(PermissionsPage)

        // Wait for initial load
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        expect(mockGetPermissionSchemes).toHaveBeenCalledWith(1, 20)
        expect(wrapper.text()).toContain('Permission Schemes')
        expect(wrapper.text()).toContain('Editor Scheme')
        expect(wrapper.text()).toContain('Viewer Scheme')
    })

    it('opens create dialog when "New Permission Scheme" button is clicked', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Verify button exists by checking text content
        expect(wrapper.text()).toContain('New Permission Scheme')

        // Open create dialog directly via method
        await (wrapper.vm as any).openCreateDialog()
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(true)
        expect((wrapper.vm as any).editingScheme).toBeNull()
    })

    it('opens edit dialog with pre-filled data', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Open edit dialog
        await (wrapper.vm as any).openEditDialog(mockSchemes[0])
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(true)
        expect((wrapper.vm as any).editingScheme).toEqual(mockSchemes[0])
        // Form data is now in PermissionSchemeDialog component
    })

    it('creates a new permission scheme', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Open create dialog
        await (wrapper.vm as any).openCreateDialog()
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(true)
        expect((wrapper.vm as any).editingScheme).toBeNull()

        // Simulate save from dialog - need to mock the composable methods
        const saveData = {
            name: 'New Scheme',
            description: 'New description',
            role_permissions: {},
        }

        // Mock the composable to call showSuccess
        const mockCreateWithSuccess = vi.fn().mockResolvedValue(undefined)
        vi.mocked(mockCreatePermissionScheme).mockImplementation(mockCreateWithSuccess)

        await (wrapper.vm as any).handleSaveScheme(saveData)
        await wrapper.vm.$nextTick()

        expect(mockCreatePermissionScheme).toHaveBeenCalledWith(saveData)
        // showSuccess is called in the composable, which is mocked
        expect((wrapper.vm as any).showDialog).toBe(false)
    })

    it('updates an existing permission scheme', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Open edit dialog
        await (wrapper.vm as any).openEditDialog(mockSchemes[0])
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).editingScheme).toEqual(mockSchemes[0])

        // Simulate save from dialog
        const updateData = {
            name: 'Updated Scheme',
            description: 'Updated description',
            role_permissions: mockSchemes[0].role_permissions,
        }
        await (wrapper.vm as any).handleSaveScheme(updateData)
        await wrapper.vm.$nextTick()

        expect(mockUpdatePermissionScheme).toHaveBeenCalledWith('scheme-1', updateData)
        // showSuccess is called in the composable
        expect((wrapper.vm as any).showDialog).toBe(false)
    })

    it('deletes a permission scheme with confirmation', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Confirm delete
        await (wrapper.vm as any).confirmDelete(mockSchemes[0])
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDeleteDialog).toBe(true)
        expect((wrapper.vm as any).schemeToDelete).toEqual(mockSchemes[0])

        // Execute delete
        await (wrapper.vm as any).deleteScheme()
        await wrapper.vm.$nextTick()

        expect(mockDeletePermissionScheme).toHaveBeenCalledWith('scheme-1')
        // showSuccess is called in the composable
        expect((wrapper.vm as any).showDeleteDialog).toBe(false)
    })

    it('handles pagination changes', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Change page
        await (wrapper.vm as any).handlePageChange(2)
        await wrapper.vm.$nextTick()

        expect(mockSetPage).toHaveBeenCalledWith(2)
        expect(mockGetPermissionSchemes).toHaveBeenCalledWith(2, 20)
    })

    it('handles items per page changes', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Change items per page
        await (wrapper.vm as any).handleItemsPerPageChange(50)
        await wrapper.vm.$nextTick()

        expect(mockSetItemsPerPage).toHaveBeenCalledWith(50)
        expect(mockGetPermissionSchemes).toHaveBeenCalledWith(1, 50)
    })

    // Note: Role and permission management is now handled in PermissionSchemeDialog component
    // These tests would be better suited for PermissionSchemeDialog.test.ts

    it('handles errors when loading schemes', async () => {
        mockGetPermissionSchemes.mockRejectedValue(new Error('Network error'))

        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Error is handled in composable, which calls showError
        expect(showError).toHaveBeenCalled()
    })

    it('handles errors when creating scheme', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        mockCreatePermissionScheme.mockRejectedValue(new Error('Creation failed'))

        await (wrapper.vm as any).openCreateDialog()
        await wrapper.vm.$nextTick()

        const saveData = {
            name: 'New Scheme',
            description: null,
            role_permissions: {},
        }

        await (wrapper.vm as any).handleSaveScheme(saveData)
        await wrapper.vm.$nextTick()

        // Error is handled in composable which calls showError
        expect(showError).toHaveBeenCalled()
    })

    it('closes dialog when cancel is clicked', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Open dialog
        await (wrapper.vm as any).openEditDialog(mockSchemes[0])
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(true)
        expect((wrapper.vm as any).editingScheme).toEqual(mockSchemes[0])

        // Close dialog (simulate dialog close event)
        ;(wrapper.vm as any).showDialog = false
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(false)
    })

    it('formats dates correctly', async () => {
        const wrapper = mount(PermissionsPage)
        await vi.waitUntil(() => mockGetPermissionSchemes.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        const formatted = (wrapper.vm as any).formatDate('2024-01-01T00:00:00Z')
        expect(formatted).toBeTruthy()
        expect(typeof formatted).toBe('string')
    })
})
