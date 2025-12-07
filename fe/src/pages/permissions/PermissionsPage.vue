<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title class="d-flex align-center justify-space-between pa-4">
                        <div class="d-flex align-center">
                            <SmartIcon
                                icon="shield"
                                :size="28"
                                class="mr-3"
                            />
                            <span class="text-h4">{{
                                t('permissions.page.title') || 'Users & Roles'
                            }}</span>
                        </div>
                    </v-card-title>

                    <v-card-text>
                        <v-tabs
                            v-model="activeTab"
                            color="primary"
                        >
                            <v-tab value="users">
                                <template #prepend>
                                    <SmartIcon
                                        icon="users"
                                        :size="20"
                                        class="mr-2"
                                    />
                                </template>
                                {{ t('permissions.page.tabs.users') || 'Users' }}
                            </v-tab>
                            <v-tab value="roles">
                                <template #prepend>
                                    <SmartIcon
                                        icon="shield"
                                        :size="20"
                                        class="mr-2"
                                    />
                                </template>
                                {{ t('permissions.page.tabs.roles') || 'Roles' }}
                            </v-tab>
                        </v-tabs>

                        <v-window v-model="activeTab">
                            <v-window-item value="users">
                                <div class="d-flex align-center justify-space-between pa-4">
                                    <h3 class="text-h6">
                                        {{ t('permissions.page.users.title') || 'Users' }}
                                    </h3>
                                    <v-btn
                                        v-if="canCreateUser"
                                        color="primary"
                                        @click="openCreateUserDialog"
                                    >
                                        <template #prepend>
                                            <SmartIcon
                                                icon="plus"
                                                :size="20"
                                            />
                                        </template>
                                        {{ t('permissions.page.users.new_button') || 'New User' }}
                                    </v-btn>
                                </div>
                                <PaginatedDataTable
                                    :items="users"
                                    :headers="userTableHeaders"
                                    :loading="usersLoading"
                                    :error="usersError"
                                    :loading-text="t('permissions.page.users.loading')"
                                    :current-page="usersCurrentPage"
                                    :items-per-page="usersItemsPerPage"
                                    :total-items="usersTotalItems"
                                    :total-pages="usersTotalPages"
                                    :has-next="usersPaginationMeta?.has_next"
                                    :has-previous="usersPaginationMeta?.has_previous"
                                    @update:page="handleUsersPageChange"
                                    @update:items-per-page="handleUsersItemsPerPageChange"
                                >
                                    <!-- Username Column -->
                                    <template #item.username="{ item }">
                                        <div class="d-flex align-center">
                                            <SmartIcon
                                                v-if="item.super_admin"
                                                icon="shield-check"
                                                color="warning"
                                                :size="20"
                                                class="mr-2"
                                            />
                                            <span>{{ item.username }}</span>
                                        </div>
                                    </template>

                                    <!-- Email Column -->
                                    <template #item.email="{ item }">
                                        <span>{{ item.email }}</span>
                                    </template>

                                    <!-- Roles Column -->
                                    <template #item.roles="{ item }">
                                        <div class="d-flex gap-1 flex-wrap">
                                            <Badge
                                                v-for="roleName in getRoleNamesForUser(item)"
                                                :key="roleName"
                                                size="small"
                                                color="primary"
                                                variant="outlined"
                                            >
                                                {{ roleName }}
                                            </Badge>
                                            <span
                                                v-if="getRoleNamesForUser(item).length === 0"
                                                class="text-body-2 text-medium-emphasis"
                                            >
                                                {{
                                                    t('permissions.page.users.no_roles') ||
                                                    'No roles'
                                                }}
                                            </span>
                                        </div>
                                    </template>

                                    <!-- Status Column -->
                                    <template #item.is_active="{ item }">
                                        <Badge
                                            size="small"
                                            :status="item.is_active ? 'success' : 'error'"
                                        >
                                            {{
                                                item.is_active
                                                    ? t('permissions.page.users.status.active')
                                                    : t('permissions.page.users.status.inactive')
                                            }}
                                        </Badge>
                                    </template>

                                    <!-- Actions Column -->
                                    <template #item.actions="{ item }">
                                        <div class="d-flex gap-2">
                                            <v-btn
                                                variant="text"
                                                size="small"
                                                color="info"
                                                @click="openEditUserDialog(item)"
                                            >
                                                <SmartIcon
                                                    icon="pencil"
                                                    :size="20"
                                                />
                                            </v-btn>
                                            <v-btn
                                                variant="text"
                                                size="small"
                                                color="error"
                                                @click="confirmDeleteUser(item)"
                                            >
                                                <SmartIcon
                                                    icon="trash-2"
                                                    :size="20"
                                                />
                                            </v-btn>
                                        </div>
                                    </template>
                                </PaginatedDataTable>
                            </v-window-item>

                            <v-window-item value="roles">
                                <div class="d-flex align-center justify-space-between pa-4">
                                    <h3 class="text-h6">
                                        {{ t('permissions.page.roles.title') || 'Roles' }}
                                    </h3>
                                    <v-btn
                                        color="primary"
                                        @click="openCreateDialog"
                                    >
                                        <template #prepend>
                                            <SmartIcon
                                                icon="plus"
                                                :size="20"
                                            />
                                        </template>
                                        {{ t('permissions.page.roles.new_button') || 'New Role' }}
                                    </v-btn>
                                </div>

                                <PaginatedDataTable
                                    :items="roles"
                                    :headers="tableHeaders"
                                    :loading="loading"
                                    :error="error"
                                    :loading-text="
                                        t('permissions.page.roles.loading') || 'Loading roles...'
                                    "
                                    :current-page="currentPage"
                                    :items-per-page="itemsPerPage"
                                    :total-items="totalItems"
                                    :total-pages="totalPages"
                                    :has-next="paginationMeta?.has_next"
                                    :has-previous="paginationMeta?.has_previous"
                                    @update:page="handlePageChange"
                                    @update:items-per-page="handleItemsPerPageChange"
                                >
                                    <!-- Name Column -->
                                    <template #item.name="{ item }">
                                        <div class="d-flex align-center">
                                            <SmartIcon
                                                v-if="item.is_system"
                                                icon="shield-lock"
                                                color="warning"
                                                :size="20"
                                                class="mr-2"
                                            />
                                            <span>{{ item.name }}</span>
                                        </div>
                                    </template>

                                    <!-- Description Column -->
                                    <template #item.description="{ item }">
                                        <span class="text-body-2 text-medium-emphasis">
                                            {{
                                                item.description ||
                                                t('permissions.page.roles.no_description') ||
                                                'No description'
                                            }}
                                        </span>
                                    </template>

                                    <!-- Permissions Count Column -->
                                    <template #item.permissions_count="{ item }">
                                        <Badge
                                            size="small"
                                            color="primary"
                                            variant="outlined"
                                        >
                                            {{ (item.permissions || []).length }}
                                            {{
                                                (item.permissions || []).length === 1
                                                    ? t('permissions.page.roles.permission') ||
                                                      'permission'
                                                    : t('permissions.page.roles.permissions') ||
                                                      'permissions'
                                            }}
                                        </Badge>
                                    </template>

                                    <!-- Created At Column -->
                                    <template #item.created_at="{ item }">
                                        <span class="text-body-2">
                                            {{ formatDate(item.created_at) }}
                                        </span>
                                    </template>

                                    <!-- Actions Column -->
                                    <template #item.actions="{ item }">
                                        <div class="d-flex gap-2">
                                            <v-btn
                                                variant="text"
                                                size="small"
                                                color="info"
                                                :disabled="item.is_system"
                                                @click="openEditDialog(item)"
                                            >
                                                <SmartIcon
                                                    icon="pencil"
                                                    :size="20"
                                                />
                                            </v-btn>
                                            <v-btn
                                                variant="text"
                                                size="small"
                                                color="error"
                                                :disabled="item.is_system"
                                                @click="confirmDelete(item)"
                                            >
                                                <SmartIcon
                                                    icon="trash-2"
                                                    :size="20"
                                                />
                                            </v-btn>
                                        </div>
                                    </template>
                                </PaginatedDataTable>
                            </v-window-item>
                        </v-window>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Create/Edit Role Dialog -->
        <RoleDialog
            v-model="showDialog"
            :editing-role="editingRole"
            :loading="saving"
            :resource-types="resourceTypes"
            :permission-types="permissionTypes"
            :access-levels="accessLevels"
            @save="handleSaveRole"
        />

        <!-- Create/Edit User Dialog -->
        <UserDialog
            v-model="showUserDialog"
            :editing-user="editingUser"
            :loading="savingUser"
            @save="handleSaveUser"
        />

        <!-- Delete Role Confirmation Dialog -->
        <DialogManager
            v-model="showDeleteDialog"
            :config="deleteDialogConfig"
            :loading="deleting"
            @confirm="deleteRole"
        >
            <p>
                {{
                    t('permissions.page.roles.delete.message', { name: roleToDelete?.name }) ||
                    `Are you sure you want to delete the role "${roleToDelete?.name}"?`
                }}
            </p>
        </DialogManager>

        <!-- Delete User Confirmation Dialog -->
        <DialogManager
            v-model="showDeleteUserDialog"
            :config="deleteUserDialogConfig"
            :loading="deletingUser"
            @confirm="deleteUser"
        >
            <p>
                {{
                    t('permissions.page.users.delete.message', { username: userToDelete?.username })
                }}
            </p>
        </DialogManager>

        <!-- Snackbar -->
        <SnackbarManager :snackbar="currentSnackbar" />
    </v-container>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted, watch, nextTick } from 'vue'
    import { useRoute } from 'vue-router'
    import { useRoles } from '@/composables/useRoles'
    import type {
        Role,
        CreateRoleRequest,
        UpdateRoleRequest,
        ResourceNamespace,
        PermissionType,
        AccessLevel,
    } from '@/types/schemas'
    import DialogManager from '@/components/common/DialogManager.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'
    import PaginatedDataTable from '@/components/tables/PaginatedDataTable.vue'
    import RoleDialog from '@/components/permissions/RoleDialog.vue'
    import UserDialog from '@/components/users/UserDialog.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import Badge from '@/components/common/Badge.vue'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { useUsers } from '@/composables/useUsers'
    import { usePagination } from '@/composables/usePagination'
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

    const { t } = useTranslations()
    const route = useRoute()

    const { currentSnackbar } = useSnackbar()
    const {
        loading,
        error,
        roles,
        loadRoles: loadRolesFromComposable,
        createRole,
        updateRole,
        deleteRole: deleteRoleFromComposable,
    } = useRoles()

    const {
        loading: usersLoading,
        error: usersError,
        users,
        loadUsers: loadUsersFromComposable,
        createUser,
        updateUser,
        deleteUser: deleteUserFromComposable,
    } = useUsers()

    const authStore = useAuthStore()

    // Permission checks
    const canCreateUser = computed(() => {
        if (authStore.isSuperAdmin) {
            return true
        }
        return authStore.hasPermission('Roles', 'Admin')
    })

    // Reactive state
    const activeTab = ref('users')
    const showDialog = ref(false)
    const showDeleteDialog = ref(false)
    const saving = ref(false)
    const deleting = ref(false)
    const editingRole = ref<Role | null>(null)
    const roleToDelete = ref<Role | null>(null)

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

    // Options
    const resourceTypes: ResourceNamespace[] = [
        'Workflows',
        'Entities',
        'EntityDefinitions',
        'ApiKeys',
        'Roles',
        'System',
    ]
    const permissionTypes: PermissionType[] = [
        'Read',
        'Create',
        'Update',
        'Delete',
        'Publish',
        'Admin',
        'Execute',
        'Custom',
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

    // User table headers
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

    // Dialog config
    const deleteDialogConfig = computed(() => ({
        title: t('permissions.page.roles.delete.title') || 'Delete Role',
        confirmText: t('permissions.page.roles.delete.confirm') || 'Delete',
        cancelText: t('permissions.page.roles.delete.cancel') || 'Cancel',
        maxWidth: '400px',
    }))

    const deleteUserDialogConfig = computed(() => ({
        title: t('permissions.page.users.delete.title'),
        confirmText: t('permissions.page.users.delete.confirm'),
        cancelText: t('permissions.page.users.delete.cancel'),
        maxWidth: '400px',
    }))

    // Helper function to get role names for a user
    const getRoleNamesForUser = (user: UserResponse): string[] => {
        if (!user.role_uuids || user.role_uuids.length === 0) {
            return []
        }
        return roles.value
            .filter(role => user.role_uuids?.includes(role.uuid))
            .map(role => role.name)
    }

    // Methods
    const loadRoles = async (page = 1, perPage = 20) => {
        try {
            const response = await loadRolesFromComposable(page, perPage)
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
            // Error already handled in composable
        }
    }

    const handlePageChange = async (page: number) => {
        currentPage.value = page
        setPage(page)
        await loadRoles(currentPage.value, itemsPerPage.value)
    }

    const handleItemsPerPageChange = async (newItemsPerPage: number) => {
        itemsPerPage.value = newItemsPerPage
        setItemsPerPage(newItemsPerPage)
        currentPage.value = 1
        setPage(1)
        await loadRoles(1, newItemsPerPage)
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
            await loadRoles(currentPage.value, itemsPerPage.value)
        } catch {
            // Error already handled in composable
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
            await loadRoles(currentPage.value, itemsPerPage.value)
        } catch {
            // Error already handled in composable
        } finally {
            deleting.value = false
        }
    }

    const formatDate = (dateString: string): string => {
        return new Date(dateString).toLocaleString()
    }

    // User management methods
    const loadUsers = async (page = 1, perPage = 20) => {
        try {
            const response = await loadUsersFromComposable(page, perPage)
            if (response.meta?.pagination) {
                usersTotalItems.value = response.meta.pagination.total
                usersTotalPages.value = response.meta.pagination.total_pages
                usersPaginationMeta.value = response.meta.pagination
            } else {
                usersTotalItems.value = users.value.length
                usersTotalPages.value = 1
                usersPaginationMeta.value = null
            }
        } catch {
            // Error already handled in composable
        }
    }

    const handleUsersPageChange = async (page: number) => {
        usersCurrentPage.value = page
        setUsersPage(page)
        await loadUsers(usersCurrentPage.value, usersItemsPerPage.value)
    }

    const handleUsersItemsPerPageChange = async (newItemsPerPage: number) => {
        usersItemsPerPage.value = newItemsPerPage
        setUsersItemsPerPage(newItemsPerPage)
        usersCurrentPage.value = 1
        setUsersPage(1)
        await loadUsers(1, newItemsPerPage)
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

        try {
            if (editingUser.value) {
                await updateUser(editingUser.value.uuid, data as UpdateUserRequest)
            } else {
                await createUser(data as CreateUserRequest)
            }

            showUserDialog.value = false
            editingUser.value = null
            await loadUsers(usersCurrentPage.value, usersItemsPerPage.value)
        } catch {
            // Error already handled in composable
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
            await loadUsers(usersCurrentPage.value, usersItemsPerPage.value)
        } catch {
            // Error already handled in composable
        } finally {
            deletingUser.value = false
        }
    }

    // Watch for tab changes to load data when switching tabs
    watch(activeTab, newTab => {
        if (newTab === 'users' && users.value.length === 0) {
            void loadUsers(usersCurrentPage.value, usersItemsPerPage.value)
        } else if (newTab === 'roles' && roles.value.length === 0) {
            void loadRoles(currentPage.value, itemsPerPage.value)
        }
    })

    // Lifecycle
    onMounted(async () => {
        void loadUsers(usersCurrentPage.value, usersItemsPerPage.value)
        void loadRoles(currentPage.value, itemsPerPage.value)

        // Check for query params
        // Switch to users tab if requested
        if (route.query.tab === 'users') {
            activeTab.value = 'users'
        }

        // Open create user dialog if requested
        if (route.query.create === 'true' && route.query.tab === 'users') {
            await nextTick()
            openCreateUserDialog()
            // Remove query params from URL
            window.history.replaceState({}, '', '/permissions')
        }
    })
</script>

<style scoped>
    .v-container {
        min-height: calc(100vh - 64px - 32px);
        overflow-x: hidden;
    }

    .v-card {
        min-height: 400px;
    }
</style>
