import { ref, computed, onMounted, nextTick, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import PaginatedDataTable from '@/shared/tables/PaginatedDataTable/index.vue'
import UserDialog from '@/modules/permissions/components/UserDialog/index.vue'
import ConfirmationDialog from '@/shared/components/ConfirmationDialog/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'
import { useUsers } from '@/modules/permissions/composables/useUsers'
import { useRoles } from '@/modules/permissions/composables/useRoles'
import { usePagination } from '@/shared/composables/usePagination'
import { useAuthStore } from '@/stores/auth'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

export default defineComponent({
    name: 'UserTab',
    components: {
        PaginatedDataTable,
        UserDialog,
        ConfirmationDialog,
        SmartIcon,
        Badge,
    },
    setup() {
        const { t } = useTranslations()
        const route = useRoute()
        const authStore = useAuthStore()

        const {
            loading: usersLoading,
            error: usersError,
            users,
            loadUsers: loadUsersFromComposable,
            createUser,
            updateUser,
            deleteUser: deleteUserFromComposable,
        } = useUsers()

        const { roles, loadRoles } = useRoles()

        // Permission checks
        const canCreateUser = computed(() => {
            return (
                authStore.hasPermission('Users', 'Create') || authStore.hasPermission('Users', 'Admin')
            )
        })

        // User management state
        const showUserDialog = ref(false)
        const showDeleteUserDialog = ref(false)
        const savingUser = ref(false)
        const deletingUser = ref(false)
        const editingUser = ref<UserResponse | null>(null)
        const userToDelete = ref<UserResponse | null>(null)

        // Users pagination
        const {
            state: usersPaginationState,
            setPage: setUsersPage,
            setItemsPerPage: setUsersItemsPerPage,
        } = usePagination('users', 20)
        const usersCurrentPage = ref(usersPaginationState.page)
        const usersItemsPerPage = ref(usersPaginationState.itemsPerPage)
        const usersTotalItems = ref(0)
        const usersTotalPages = ref(1)
        const usersPaginationMeta = ref<{
            total: number
            page: number
            per_page: number
            total_pages: number
            has_previous: boolean
            has_next: boolean
        } | null>(null)

        const usersSortBy = ref<string | null>(null)
        const usersSortOrder = ref<'asc' | 'desc' | null>(null)

        // Table headers
        const userTableHeaders = computed(() => [
            {
                title: t('permissions.page.users.table.username') || 'Username',
                key: 'username',
                sortable: true,
            },
            { title: t('permissions.page.users.table.email') || 'Email', key: 'email', sortable: true },
            { title: t('permissions.page.users.table.roles') || 'Roles', key: 'roles', sortable: true },
            {
                title: t('permissions.page.users.table.status') || 'Status',
                key: 'is_active',
                sortable: true,
            },
            {
                title: t('permissions.page.users.table.created') || 'Created',
                key: 'created_at',
                sortable: true,
            },
            {
                title: t('permissions.page.users.table.actions') || 'Actions',
                key: 'actions',
                sortable: false,
            },
        ])

        const deleteUserDialogConfig = computed(() => ({
            title: t('permissions.page.users.delete.title'),
            confirmText: t('permissions.page.users.delete.confirm'),
            cancelText: t('permissions.page.users.delete.cancel'),
            maxWidth: '400px',
        }))

        // Helper functions
        const getRoleNamesForUser = (user: UserResponse): string[] => {
            if (user.role_uuids.length === 0) {
                return []
            }
            return roles.value
                .filter(role => user.role_uuids.includes(role.uuid))
                .map(role => role.name)
        }

        const formatDate = (dateString: string): string => {
            return new Date(dateString).toLocaleString()
        }

        // Methods
        const loadData = async (page = 1, perPage = 20) => {
            try {
                const [userResponse] = await Promise.all([
                    loadUsersFromComposable(page, perPage, usersSortBy.value, usersSortOrder.value),
                    roles.value.length === 0 ? loadRoles(1, 100) : Promise.resolve(null),
                ])

                if (userResponse?.meta?.pagination) {
                    usersTotalItems.value = userResponse.meta.pagination.total
                    usersTotalPages.value = userResponse.meta.pagination.total_pages
                    usersPaginationMeta.value = userResponse.meta.pagination
                } else {
                    usersTotalItems.value = users.value.length
                    usersTotalPages.value = 1
                    usersPaginationMeta.value = null
                }
            } catch {
                // Error handled in composable
            }
        }

        const handleUsersPageChange = async (page: number) => {
            usersCurrentPage.value = page
            setUsersPage(page)
            await loadData(usersCurrentPage.value, usersItemsPerPage.value)
        }

        const handleUsersItemsPerPageChange = async (newItemsPerPage: number) => {
            usersItemsPerPage.value = newItemsPerPage
            setUsersItemsPerPage(newItemsPerPage)
            usersCurrentPage.value = 1
            setUsersPage(1)
            await loadData(1, newItemsPerPage)
        }

        const handleUsersSortChange = async (
            newSortBy: string | null,
            newSortOrder: 'asc' | 'desc' | null
        ) => {
            usersSortBy.value = newSortBy
            usersSortOrder.value = newSortOrder
            usersCurrentPage.value = 1
            setUsersPage(1)
            await loadData(1, usersItemsPerPage.value)
        }

        const openCreateUserDialog = () => {
            editingUser.value = null
            showUserDialog.value = true
        }

        const openEditUserDialog = (user: UserResponse) => {
            editingUser.value = user
            showUserDialog.value = true
        }

        const handleSaveUser = async (data: CreateUserRequest | UpdateUserRequest) => {
            savingUser.value = true
            const isEditing = !!editingUser.value

            try {
                if (isEditing) {
                    await updateUser(editingUser.value!.uuid, data as UpdateUserRequest)
                } else {
                    await createUser(data as CreateUserRequest)
                }

                showUserDialog.value = false
                editingUser.value = null
                await loadData(usersCurrentPage.value, usersItemsPerPage.value)
            } catch {
                // Error handled in composable
            } finally {
                savingUser.value = false
            }
        }

        const confirmDeleteUser = (user: UserResponse) => {
            userToDelete.value = user
            showDeleteUserDialog.value = true
        }

        const deleteUser = async () => {
            if (!userToDelete.value) {
                return
            }

            deletingUser.value = true

            try {
                await deleteUserFromComposable(userToDelete.value.uuid)
                showDeleteUserDialog.value = false
                userToDelete.value = null
                await loadData(usersCurrentPage.value, usersItemsPerPage.value)
            } catch {
                // Error handled in composable
            } finally {
                deletingUser.value = false
            }
        }

        // Lifecycle
        onMounted(async () => {
            void loadData(usersCurrentPage.value, usersItemsPerPage.value)

            if (route.query.create === 'true' && route.query.tab === 'users') {
                await nextTick()
                openCreateUserDialog()
                window.history.replaceState({}, '', '/permissions')
            }
        })

        return {
            t,
            canCreateUser,
            users,
            userTableHeaders,
            usersLoading,
            usersError,
            usersCurrentPage,
            usersItemsPerPage,
            usersTotalItems,
            usersTotalPages,
            usersPaginationMeta,
            showUserDialog,
            editingUser,
            savingUser,
            showDeleteUserDialog,
            deleteUserDialogConfig,
            deletingUser,
            userToDelete,
            handleUsersPageChange,
            handleUsersItemsPerPageChange,
            handleUsersSortChange,
            openCreateUserDialog,
            openEditUserDialog,
            handleSaveUser,
            confirmDeleteUser,
            deleteUser,
            getRoleNamesForUser,
            formatDate,
        }
    },
})
