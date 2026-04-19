import { ref, computed, onMounted, nextTick, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import PaginatedDataTable from '@/shared/tables/PaginatedDataTable/index.vue'
import RoleDialog from '@/modules/permissions/components/RoleDialog/index.vue'
import ConfirmationDialog from '@/shared/components/ConfirmationDialog/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'
import { useRoles } from '@/modules/permissions/composables/useRoles'
import { usePagination } from '@/shared/composables/usePagination'
import { useAuthStore } from '@/stores/auth'
import { useTranslations } from '@/shared/composables/useTranslations'
import type {
    Role,
    CreateRoleRequest,
    UpdateRoleRequest,
    ResourceNamespace,
    PermissionType,
    AccessLevel,
} from '@/types/schemas'

export default defineComponent({
    name: 'RoleTab',
    components: {
        PaginatedDataTable,
        RoleDialog,
        ConfirmationDialog,
        SmartIcon,
        Badge,
    },
    setup() {
        const { t } = useTranslations()
        const route = useRoute()
        const authStore = useAuthStore()

        const {
            loading,
            error,
            roles,
            loadRoles: loadRolesFromComposable,
            createRole,
            updateRole,
            deleteRole: deleteRoleFromComposable,
        } = useRoles()

        // Permission checks
        const canCreateRole = computed(() => {
            return (
                authStore.hasPermission('Roles', 'Create') || authStore.hasPermission('Roles', 'Admin')
            )
        })

        // Role management state
        const showDialog = ref(false)
        const showDeleteDialog = ref(false)
        const saving = ref(false)
        const deleting = ref(false)
        const editingRole = ref<Role | null>(null)
        const roleToDelete = ref<Role | null>(null)

        // Pagination
        const { state: paginationState, setPage, setItemsPerPage } = usePagination('permissions', 20)
        const currentPage = ref(paginationState.page)
        const itemsPerPage = ref(paginationState.itemsPerPage)
        const totalItems = ref(0)
        const totalPages = ref(1)
        const paginationMeta = ref<{
            total: number
            page: number
            per_page: number
            total_pages: number
            has_previous: boolean
            has_next: boolean
        } | null>(null)
        const sortBy = ref<string | null>(null)
        const sortOrder = ref<'asc' | 'desc' | null>(null)

        // Options
        const resourceTypes: ResourceNamespace[] = [
            'Workflows',
            'Entities',
            'EntityDefinitions',
            'ApiKeys',
            'Roles',
            'Users',
            'System',
            'DashboardStats',
        ]
        const permissionTypes: PermissionType[] = [
            'Read',
            'Create',
            'Update',
            'Delete',
            'Publish',
            'Admin',
            'Execute',
        ]
        const accessLevels: AccessLevel[] = ['None', 'Own', 'Group', 'All']

        // Table headers
        const tableHeaders = computed(() => [
            { title: t('permissions.page.roles.table.name') || 'Name', key: 'name', sortable: true },
            {
                title: t('permissions.page.roles.table.description') || 'Description',
                key: 'description',
                sortable: false,
            },
            {
                title: t('permissions.page.roles.table.permissions') || 'Permissions',
                key: 'permissions_count',
                sortable: false,
            },
            {
                title: t('permissions.page.roles.table.created') || 'Created',
                key: 'created_at',
                sortable: true,
            },
            {
                title: t('permissions.page.roles.table.actions') || 'Actions',
                key: 'actions',
                sortable: false,
            },
        ])

        const deleteDialogConfig = computed(() => ({
            title: t('permissions.page.roles.delete.title') || 'Delete Role',
            confirmText: t('permissions.page.roles.delete.confirm') || 'Delete',
            cancelText: t('permissions.page.roles.delete.cancel') || 'Cancel',
            maxWidth: '400px',
        }))

        // Methods
        const loadRolesData = async (page = 1, perPage = 20) => {
            try {
                const response = await loadRolesFromComposable(
                    page,
                    perPage,
                    sortBy.value,
                    sortOrder.value
                )
                if (response.meta?.pagination) {
                    totalItems.value = response.meta.pagination.total
                    totalPages.value = response.meta.pagination.total_pages
                    paginationMeta.value = response.meta.pagination
                } else {
                    totalItems.value = roles.value.length
                    totalPages.value = 1
                    paginationMeta.value = null
                }
            } catch {
                // Error handled in composable
            }
        }

        const handlePageChange = async (page: number) => {
            currentPage.value = page
            setPage(page)
            await loadRolesData(currentPage.value, itemsPerPage.value)
        }

        const handleItemsPerPageChange = async (newItemsPerPage: number) => {
            itemsPerPage.value = newItemsPerPage
            setItemsPerPage(newItemsPerPage)
            currentPage.value = 1
            setPage(1)
            await loadRolesData(1, newItemsPerPage)
        }

        const handleSortChange = async (
            newSortBy: string | null,
            newSortOrder: 'asc' | 'desc' | null
        ) => {
            sortBy.value = newSortBy
            sortOrder.value = newSortOrder
            currentPage.value = 1
            setPage(1)
            await loadRolesData(1, itemsPerPage.value)
        }

        const openCreateDialog = () => {
            editingRole.value = null
            showDialog.value = true
        }

        const openEditDialog = (role: Role) => {
            editingRole.value = role
            showDialog.value = true
        }

        const handleSaveRole = async (data: CreateRoleRequest | UpdateRoleRequest) => {
            saving.value = true

            try {
                if (editingRole.value) {
                    await updateRole(editingRole.value.uuid, data as UpdateRoleRequest)
                } else {
                    await createRole(data as CreateRoleRequest)
                }

                showDialog.value = false
                editingRole.value = null
                await loadRolesData(currentPage.value, itemsPerPage.value)
            } catch {
                // Error handled in composable
            } finally {
                saving.value = false
            }
        }

        const confirmDelete = (role: Role) => {
            roleToDelete.value = role
            showDeleteDialog.value = true
        }

        const deleteRole = async () => {
            if (!roleToDelete.value) {
                return
            }

            deleting.value = true

            try {
                await deleteRoleFromComposable(roleToDelete.value.uuid)
                showDeleteDialog.value = false
                roleToDelete.value = null
                await loadRolesData(currentPage.value, itemsPerPage.value)
            } catch {
                // Error handled in composable
            } finally {
                deleting.value = false
            }
        }

        const formatDate = (dateString: string): string => {
            return new Date(dateString).toLocaleString()
        }

        // Lifecycle
        onMounted(async () => {
            void loadRolesData(currentPage.value, itemsPerPage.value)

            if (route.query.create === 'true' && route.query.tab === 'roles') {
                await nextTick()
                openCreateDialog()
                window.history.replaceState({}, '', '/permissions')
            }
        })

        return {
            t,
            canCreateRole,
            roles,
            tableHeaders,
            loading,
            error,
            currentPage,
            itemsPerPage,
            totalItems,
            totalPages,
            paginationMeta,
            showDialog,
            editingRole,
            saving,
            showDeleteDialog,
            deleteDialogConfig,
            deleting,
            roleToDelete,
            resourceTypes,
            permissionTypes,
            accessLevels,
            handlePageChange,
            handleItemsPerPageChange,
            handleSortChange,
            openCreateDialog,
            openEditDialog,
            handleSaveRole,
            confirmDelete,
            deleteRole,
            formatDate,
        }
    },
})
