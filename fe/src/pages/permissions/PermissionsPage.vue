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
                            <span class="text-h4">Permission Schemes</span>
                        </div>
                        <v-btn
                            color="primary"
                            prepend-icon="mdi-plus"
                            @click="openCreateDialog"
                        >
                            New Permission Scheme
                        </v-btn>
                    </v-card-title>

                    <v-card-text>
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
                                    {{ Object.keys(item.role_permissions || {}).length }} roles
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
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Create/Edit Dialog -->
        <PermissionSchemeDialog
            v-model="showDialog"
            :editing-scheme="editingScheme"
            :loading="saving"
            :resource-types="resourceTypes"
            :permission-types="permissionTypes"
            :access-levels="accessLevels"
            @save="handleSaveScheme"
        />

        <!-- Delete Confirmation Dialog -->
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

        <!-- Snackbar -->
        <SnackbarManager :snackbar="currentSnackbar" />
    </v-container>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
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
    import { useSnackbar } from '@/composables/useSnackbar'

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

    // Reactive state
    const showDialog = ref(false)
    const showDeleteDialog = ref(false)
    const saving = ref(false)
    const deleting = ref(false)
    const editingScheme = ref<PermissionScheme | null>(null)
    const schemeToDelete = ref<PermissionScheme | null>(null)

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

    // Dialog config
    const deleteDialogConfig = computed(() => ({
        title: 'Delete Permission Scheme',
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
