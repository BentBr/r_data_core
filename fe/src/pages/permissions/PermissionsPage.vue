<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title class="d-flex align-center justify-space-between pa-4">
                        <div class="d-flex align-center">
                            <v-icon
                                icon="mdi-shield-account"
                                class="mr-3"
                            />
                            <span class="text-h4">Users and Roles</span>
                        </div>
                    </v-card-title>

                    <v-card-text>
                        <v-tabs
                            v-model="activeTab"
                            bg-color="primary"
                        >
                            <v-tab value="schemes">
                                <v-icon
                                    icon="mdi-shield-account"
                                    class="mr-2"
                                />
                                Permission Schemes
                            </v-tab>
                            <v-tab value="users">
                                <v-icon
                                    icon="mdi-account-group"
                                    class="mr-2"
                                />
                                Users and Roles
                            </v-tab>
                        </v-tabs>

                        <v-window v-model="activeTab">
                            <v-window-item value="schemes">
                                <div class="d-flex align-center justify-space-between pa-4">
                                    <h3 class="text-h6">Permission Schemes</h3>
                                    <v-btn
                                        color="primary"
                                        prepend-icon="mdi-plus"
                                        @click="openCreateDialog"
                                    >
                                        New Permission Scheme
                                    </v-btn>
                                </div>

                                <PaginatedDataTable
                                    :items="schemes"
                                    :headers="tableHeaders"
                                    :loading="loading"
                                    :error="error"
                                    loading-text="Loading permission schemes..."
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
                                            <v-icon
                                                v-if="item.is_system"
                                                icon="mdi-shield-lock"
                                                color="warning"
                                                class="mr-2"
                                            />
                                            <span>{{ item.name }}</span>
                                        </div>
                                    </template>

                                    <!-- Description Column -->
                                    <template #item.description="{ item }">
                                        <span class="text-body-2 text-medium-emphasis">
                                            {{ item.description || 'No description' }}
                                        </span>
                                    </template>

                                    <!-- Roles Count Column -->
                                    <template #item.roles_count="{ item }">
                                        <v-chip
                                            size="small"
                                            color="primary"
                                            variant="outlined"
                                        >
                                            {{ Object.keys(item.role_permissions || {}).length }}
                                            roles
                                        </v-chip>
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
                                                icon="mdi-pencil"
                                                variant="text"
                                                size="small"
                                                color="info"
                                                :disabled="item.is_system"
                                                @click="openEditDialog(item)"
                                            />
                                            <v-btn
                                                icon="mdi-delete"
                                                variant="text"
                                                size="small"
                                                color="error"
                                                :disabled="item.is_system"
                                                @click="confirmDelete(item)"
                                            />
                                        </div>
                                    </template>
                                </PaginatedDataTable>
                            </v-window-item>

                            <v-window-item value="users">
                                <div class="d-flex align-center justify-space-between pa-4">
                                    <h3 class="text-h6">Users and Roles</h3>
                                    <v-btn
                                        v-if="canCreateUser"
                                        color="primary"
                                        prepend-icon="mdi-plus"
                                        @click="openCreateUserDialog"
                                    >
                                        New User
                                    </v-btn>
                                </div>
                                <PaginatedDataTable
                                    :items="users"
                                    :headers="userTableHeaders"
                                    :loading="usersLoading"
                                    :error="usersError"
                                    loading-text="Loading users..."
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
                                            <v-icon
                                                v-if="item.super_admin"
                                                icon="mdi-shield-star"
                                                color="warning"
                                                class="mr-2"
                                            />
                                            <span>{{ item.username }}</span>
                                        </div>
                                    </template>

                                    <!-- Email Column -->
                                    <template #item.email="{ item }">
                                        <span>{{ item.email }}</span>
                                    </template>

                                    <!-- Role Column -->
                                    <template #item.role="{ item }">
                                        <v-chip
                                            size="small"
                                            :color="item.super_admin ? 'warning' : 'primary'"
                                            variant="outlined"
                                        >
                                            {{ item.role }}
                                        </v-chip>
                                    </template>

                                    <!-- Status Column -->
                                    <template #item.is_active="{ item }">
                                        <v-chip
                                            size="small"
                                            :color="item.is_active ? 'success' : 'error'"
                                        >
                                            {{ item.is_active ? 'Active' : 'Inactive' }}
                                        </v-chip>
                                    </template>

                                    <!-- Actions Column -->
                                    <template #item.actions="{ item }">
                                        <div class="d-flex gap-2">
                                            <v-btn
                                                icon="mdi-pencil"
                                                variant="text"
                                                size="small"
                                                color="info"
                                                @click="openEditUserDialog(item)"
                                            />
                                            <v-btn
                                                icon="mdi-delete"
                                                variant="text"
                                                size="small"
                                                color="error"
                                                @click="confirmDeleteUser(item)"
                                            />
                                        </div>
                                    </template>
                                </PaginatedDataTable>
                            </v-window-item>
                        </v-window>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Create/Edit Permission Scheme Dialog -->
        <PermissionSchemeDialog
            v-model="showDialog"
            :editing-scheme="editingScheme"
            :loading="saving"
            :resource-types="resourceTypes"
            :permission-types="permissionTypes"
            :access-levels="accessLevels"
            @save="handleSaveScheme"
        />

        <!-- Create/Edit User Dialog -->
        <UserDialog
            v-model="showUserDialog"
            :editing-user="editingUser"
            :loading="savingUser"
            @save="handleSaveUser"
        />

        <!-- Delete Permission Scheme Confirmation Dialog -->
        <DialogManager
            v-model="showDeleteDialog"
            :config="deleteDialogConfig"
            :loading="deleting"
            @confirm="deleteScheme"
        >
            <p>
                Are you sure you want to delete the permission scheme "{{ schemeToDelete?.name }}"?
            </p>
        </DialogManager>

        <!-- Delete User Confirmation Dialog -->
        <DialogManager
            v-model="showDeleteUserDialog"
            :config="deleteUserDialogConfig"
            :loading="deletingUser"
            @confirm="deleteUser"
        >
            <p>Are you sure you want to delete the user "{{ userToDelete?.username }}"?</p>
        </DialogManager>

        <!-- Snackbar -->
        <SnackbarManager :snackbar="currentSnackbar" />
    </v-container>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted, watch } from 'vue'
    import { usePagination } from '@/composables/usePagination'
    import { usePermissionSchemes } from '@/composables/usePermissionSchemes'
    import type {
        PermissionScheme,
        CreatePermissionSchemeRequest,
        UpdatePermissionSchemeRequest,
        ResourceNamespace,
        PermissionType,
        AccessLevel,
    } from '@/types/schemas'
    import DialogManager from '@/components/common/DialogManager.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'
    import PaginatedDataTable from '@/components/tables/PaginatedDataTable.vue'
    import PermissionSchemeDialog from '@/components/permissions/PermissionSchemeDialog.vue'
    import UserDialog from '@/components/users/UserDialog.vue'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { useUsers } from '@/composables/useUsers'
    import { usePagination } from '@/composables/usePagination'
    import { useAuthStore } from '@/stores/auth'
    import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

    const { currentSnackbar } = useSnackbar()
    const {
        loading,
        error,
        schemes,
        loadSchemes: loadSchemesFromComposable,
        createScheme,
        updateScheme,
        deleteScheme: deleteSchemeFromComposable,
    } = usePermissionSchemes()

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
    const canCreateScheme = computed(
        () => authStore.isSuperAdmin || authStore.hasPermission('PermissionSchemes', 'Create')
    )
    const canCreateUser = computed(
        () => authStore.isSuperAdmin || authStore.hasPermission('PermissionSchemes', 'Admin')
    )

    // Reactive state
    const activeTab = ref('schemes')
    const showDialog = ref(false)
    const showDeleteDialog = ref(false)
    const saving = ref(false)
    const deleting = ref(false)
    const editingScheme = ref<PermissionScheme | null>(null)
    const schemeToDelete = ref<PermissionScheme | null>(null)

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
        'PermissionSchemes',
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
        { title: 'Name', key: 'name', sortable: true },
        { title: 'Description', key: 'description', sortable: false },
        { title: 'Roles', key: 'roles_count', sortable: false },
        { title: 'Created', key: 'created_at', sortable: true },
        { title: 'Actions', key: 'actions', sortable: false },
    ])

    // User table headers
    const userTableHeaders = computed(() => [
        { title: 'Username', key: 'username', sortable: true },
        { title: 'Email', key: 'email', sortable: true },
        { title: 'Role', key: 'role', sortable: true },
        { title: 'Status', key: 'is_active', sortable: true },
        { title: 'Created', key: 'created_at', sortable: true },
        { title: 'Actions', key: 'actions', sortable: false },
    ])

    // Dialog config
    const deleteDialogConfig = computed(() => ({
        title: 'Delete Permission Scheme',
        confirmText: 'Delete',
        cancelText: 'Cancel',
        maxWidth: '400px',
    }))

    const deleteUserDialogConfig = computed(() => ({
        title: 'Delete User',
        confirmText: 'Delete',
        cancelText: 'Cancel',
        maxWidth: '400px',
    }))

    // Methods
    const loadSchemes = async (page = 1, perPage = 20) => {
        try {
            const response = await loadSchemesFromComposable(page, perPage)
            if (response.meta?.pagination) {
                totalItems.value = response.meta.pagination.total
                totalPages.value = response.meta.pagination.total_pages
                paginationMeta.value = response.meta.pagination
            } else {
                totalItems.value = schemes.value.length
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
        await loadSchemes(currentPage.value, itemsPerPage.value)
    }

    const handleItemsPerPageChange = async (newItemsPerPage: number) => {
        itemsPerPage.value = newItemsPerPage
        setItemsPerPage(newItemsPerPage)
        currentPage.value = 1
        setPage(1)
        await loadSchemes(1, newItemsPerPage)
    }

    const openCreateDialog = () => {
        editingScheme.value = null
        showDialog.value = true
    }

    const openEditDialog = (scheme: PermissionScheme) => {
        editingScheme.value = scheme
        showDialog.value = true
    }

    const handleSaveScheme = async (
        data: CreatePermissionSchemeRequest | UpdatePermissionSchemeRequest
    ) => {
        saving.value = true

        try {
            if (editingScheme.value) {
                await updateScheme(editingScheme.value.uuid, data as UpdatePermissionSchemeRequest)
            } else {
                await createScheme(data as CreatePermissionSchemeRequest)
            }

            showDialog.value = false
            editingScheme.value = null
            await loadSchemes(currentPage.value, itemsPerPage.value)
        } catch {
            // Error already handled in composable
        } finally {
            saving.value = false
        }
    }

    const confirmDelete = (scheme: PermissionScheme) => {
        schemeToDelete.value = scheme
        showDeleteDialog.value = true
    }

    const deleteScheme = async () => {
        if (!schemeToDelete.value) {
            return
        }

        deleting.value = true

        try {
            await deleteSchemeFromComposable(schemeToDelete.value.uuid)
            showDeleteDialog.value = false
            schemeToDelete.value = null
            await loadSchemes(currentPage.value, itemsPerPage.value)
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

    // Watch for tab changes to load users when switching to users tab
    watch(activeTab, newTab => {
        if (newTab === 'users' && users.value.length === 0) {
            void loadUsers(usersCurrentPage.value, usersItemsPerPage.value)
        }
    })

    // Lifecycle
    onMounted(() => {
        void loadSchemes(currentPage.value, itemsPerPage.value)
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
