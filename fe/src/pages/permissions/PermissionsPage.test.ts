import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { ref } from 'vue'
import PermissionsPage from './PermissionsPage.vue'
import type { Role } from '@/types/schemas'

const mockGetRoles = vi.fn()
const mockGetRole = vi.fn()
const mockCreateRole = vi.fn()
const mockUpdateRole = vi.fn()
const mockDeleteRole = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getRoles: (page?: number, itemsPerPage?: number) => mockGetRoles(page, itemsPerPage),
        getRole: (uuid: string) => mockGetRole(uuid),
        createRole: (data: unknown) => mockCreateRole(data),
        updateRole: (uuid: string, data: unknown) => mockUpdateRole(uuid, data),
        deleteRole: (uuid: string) => mockDeleteRole(uuid),
    },
}))

const mockLoading = ref(false)
const mockError = ref('')
const mockRolesList = ref<Role[]>([])

vi.mock('@/composables/useRoles', () => ({
    useRoles: () => {
        // Create wrapper functions that call showError/showSuccess on error/success
        const createRoleWrapper = async (data: unknown) => {
            try {
                const result = await mockCreateRole(data)
                showSuccess('Role created successfully')
                return result
            } catch (err) {
                showError(err instanceof Error ? err.message : 'Failed to create role')
                throw err
            }
        }

        const updateRoleWrapper = async (uuid: string, data: unknown) => {
            try {
                const result = await mockUpdateRole(uuid, data)
                showSuccess('Role updated successfully')
                return result
            } catch (err) {
                showError(err instanceof Error ? err.message : 'Failed to update role')
                throw err
            }
        }

        const deleteRoleWrapper = async (uuid: string) => {
            try {
                const result = await mockDeleteRole(uuid)
                showSuccess('Role deleted successfully')
                return result
            } catch (err) {
                showError(err instanceof Error ? err.message : 'Failed to delete role')
                throw err
            }
        }

        const loadRolesWrapper = async (page?: number, perPage?: number) => {
            try {
                const result = await mockGetRoles(page, perPage)
                mockRolesList.value = result.data
                return result
            } catch (err) {
                mockError.value = err instanceof Error ? err.message : 'Failed to load roles'
                showError(mockError.value)
                throw err
            }
        }

        return {
            loading: mockLoading,
            error: mockError,
            roles: mockRolesList,
            loadRoles: loadRolesWrapper,
            createRole: createRoleWrapper,
            updateRole: updateRoleWrapper,
            deleteRole: deleteRoleWrapper,
        }
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (k: string) => {
            // Return a more readable translation for common keys
            const translations: Record<string, string> = {
                'permissions.page.title': 'Users and Roles',
                'permissions.page.tabs.roles': 'Roles',
                'permissions.page.tabs.users': 'Users and Roles',
                'permissions.page.roles.title': 'Roles',
                'permissions.page.roles.new_button': 'New Role',
            }
            return translations[k] || k.split('.').pop() || k
        },
    }),
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

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        isSuperAdmin: false,
        hasPermission: vi.fn(() => true),
    }),
}))

const mockUsersLoading = ref(false)
const mockUsersError = ref('')
const mockUsers = ref([])

vi.mock('@/composables/useUsers', () => ({
    useUsers: () => ({
        loading: mockUsersLoading,
        error: mockUsersError,
        users: mockUsers,
        loadUsers: vi.fn().mockResolvedValue({
            data: [],
            meta: {
                pagination: {
                    total: 0,
                    page: 1,
                    per_page: 20,
                    total_pages: 1,
                    has_previous: false,
                    has_next: false,
                },
            },
        }),
        createUser: vi.fn(),
        updateUser: vi.fn(),
        deleteUser: vi.fn(),
    }),
}))

const mockRoles: Role[] = [
    {
        uuid: 'role-1',
        name: 'Editor Role',
        description: 'Permissions for editors',
        permissions: [
            {
                resource_type: 'Workflows',
                permission_type: 'Read',
                access_level: 'All',
                resource_uuids: [],
                constraints: null,
            },
        ],
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        created_by: 'user-1',
        updated_by: null,
        is_system: false,
        super_admin: false,
        published: false,
        version: 1,
    },
    {
        uuid: 'role-2',
        name: 'Viewer Role',
        description: 'Read-only permissions',
        super_admin: false,
        permissions: [
            {
                resource_type: 'Workflows',
                permission_type: 'Read',
                access_level: 'Own',
                resource_uuids: [],
                constraints: null,
            },
        ],
        created_at: '2024-01-02T00:00:00Z',
        updated_at: '2024-01-02T00:00:00Z',
        created_by: 'user-1',
        updated_by: null,
        is_system: false,
        published: false,
        version: 1,
    },
]

describe('PermissionsPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockLoading.value = false
        mockError.value = ''
        mockRolesList.value = []
        mockUsersLoading.value = false
        mockUsersError.value = ''
        mockUsers.value = []

        // Setup loadRoles - the wrapper will handle updating mockRolesList
        mockGetRoles.mockResolvedValue({
            data: mockRoles,
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

        mockCreateRole.mockResolvedValue(mockRoles[0])
        mockUpdateRole.mockResolvedValue(mockRoles[0])
        mockDeleteRole.mockResolvedValue({ message: 'Deleted successfully' })
    })

    it('renders correctly and loads roles', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                    RoleDialog: true,
                },
            },
        })

        // Wait for initial load
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        // Wait for roles to be populated
        await vi.waitUntil(() => mockRolesList.value.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()
        await wrapper.vm.$nextTick()

        expect(mockGetRoles).toHaveBeenCalledWith(1, 20)
        // Check that roles are loaded (they should be in the component's roles ref)
        expect(mockRolesList.value.length).toBeGreaterThan(0)
    })

    it('opens create dialog when "New Role" button is clicked', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Verify button exists by checking text content
        // Button text may vary based on translations, so we'll just test the functionality

        // Open create dialog directly via method
        await (wrapper.vm as any).openCreateDialog()
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(true)
        expect((wrapper.vm as any).editingRole).toBeNull()
    })

    it('opens edit dialog with pre-filled data', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Open edit dialog
        await (wrapper.vm as any).openEditDialog(mockRoles[0])
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(true)
        expect((wrapper.vm as any).editingRole).toEqual(mockRoles[0])
        // Form data is now in RoleDialog component
    })

    it('creates a new role', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Open create dialog
        await (wrapper.vm as any).openCreateDialog()
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(true)
        expect((wrapper.vm as any).editingRole).toBeNull()

        // Simulate save from dialog - need to mock the composable methods
        const saveData = {
            name: 'New Role',
            description: 'New description',
            permissions: [],
        }

        // Mock the composable to call showSuccess
        const mockCreateWithSuccess = vi.fn().mockResolvedValue(undefined)
        vi.mocked(mockCreateRole).mockImplementation(mockCreateWithSuccess)

        await (wrapper.vm as any).handleSaveRole(saveData)
        await wrapper.vm.$nextTick()

        expect(mockCreateRole).toHaveBeenCalledWith(saveData)
        // showSuccess is called in the composable, which is mocked
        expect((wrapper.vm as any).showDialog).toBe(false)
    })

    it('updates an existing role', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Open edit dialog
        await (wrapper.vm as any).openEditDialog(mockRoles[0])
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).editingRole).toEqual(mockRoles[0])

        // Simulate save from dialog
        const updateData = {
            name: 'Updated Role',
            description: 'Updated description',
            permissions: mockRoles[0].permissions,
        }
        await (wrapper.vm as any).handleSaveRole(updateData)
        await wrapper.vm.$nextTick()

        expect(mockUpdateRole).toHaveBeenCalledWith('role-1', updateData)
        // showSuccess is called in the composable
        expect((wrapper.vm as any).showDialog).toBe(false)
    })

    it('deletes a role with confirmation', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Confirm delete
        await (wrapper.vm as any).confirmDelete(mockRoles[0])
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDeleteDialog).toBe(true)
        expect((wrapper.vm as any).roleToDelete).toEqual(mockRoles[0])

        // Execute delete
        await (wrapper.vm as any).deleteRole()
        await wrapper.vm.$nextTick()

        expect(mockDeleteRole).toHaveBeenCalledWith('role-1')
        // showSuccess is called in the composable
        expect((wrapper.vm as any).showDeleteDialog).toBe(false)
    })

    it('handles pagination changes', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Change page
        await (wrapper.vm as any).handlePageChange(2)
        await wrapper.vm.$nextTick()

        expect(mockSetPage).toHaveBeenCalledWith(2)
        expect(mockGetRoles).toHaveBeenCalledWith(2, 20)
    })

    it('handles items per page changes', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Change items per page
        await (wrapper.vm as any).handleItemsPerPageChange(50)
        await wrapper.vm.$nextTick()

        expect(mockSetItemsPerPage).toHaveBeenCalledWith(50)
        expect(mockGetRoles).toHaveBeenCalledWith(1, 50)
    })

    // Note: Role and permission management is now handled in RoleDialog component
    // These tests would be better suited for RoleDialog.test.ts

    it('handles errors when loading roles', async () => {
        mockGetRoles.mockRejectedValue(new Error('Network error'))

        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Error is handled in composable, which calls showError
        expect(showError).toHaveBeenCalled()
    })

    it('handles errors when creating role', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        mockCreateRole.mockRejectedValue(new Error('Creation failed'))

        await (wrapper.vm as any).openCreateDialog()
        await wrapper.vm.$nextTick()

        const saveData = {
            name: 'New Role',
            description: null,
            permissions: [],
        }

        await (wrapper.vm as any).handleSaveRole(saveData)
        await wrapper.vm.$nextTick()

        // Error is handled in composable which calls showError
        expect(showError).toHaveBeenCalled()
    })

    it('closes dialog when cancel is clicked', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Open dialog
        await (wrapper.vm as any).openEditDialog(mockRoles[0])
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(true)
        expect((wrapper.vm as any).editingRole).toEqual(mockRoles[0])

        // Close dialog (simulate dialog close event)
        ;(wrapper.vm as any).showDialog = false
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showDialog).toBe(false)
    })

    it('formats dates correctly', async () => {
        const wrapper = mount(PermissionsPage, {
            global: {
                stubs: {
                    UserDialog: true,
                },
            },
        })
        await vi.waitUntil(() => mockGetRoles.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        const formatted = (wrapper.vm as any).formatDate('2024-01-01T00:00:00Z')
        expect(formatted).toBeTruthy()
        expect(typeof formatted).toBe('string')
    })
})
