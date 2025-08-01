<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title class="d-flex align-center justify-space-between pa-4">
                        <div class="d-flex align-center">
                            <v-icon
                                icon="mdi-key"
                                class="mr-3"
                            />
                            <span class="text-h4">API Keys Management</span>
                        </div>
                        <v-btn
                            color="primary"
                            prepend-icon="mdi-key-plus"
                            @click="showCreateDialog = true"
                        >
                            {{ t('api_keys.create.button') }}
                        </v-btn>
                    </v-card-title>

                    <v-card-text>
                        <PaginatedDataTable
                            :items="apiKeys"
                            :headers="tableHeaders"
                            :loading="loading"
                            :error="error"
                            loading-text="Loading API keys..."
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
                                        :icon="item.is_active ? 'mdi-key' : 'mdi-key-off'"
                                        :color="item.is_active ? 'success' : 'error'"
                                        class="mr-2"
                                    />
                                    <span
                                        :class="{ 'text-decoration-line-through': !item.is_active }"
                                    >
                                        {{ item.name }}
                                    </span>
                                </div>
                            </template>

                            <!-- Description Column -->
                            <template #item.description="{ item }">
                                <span class="text-body-2 text-medium-emphasis">
                                    {{ item.description || 'No description' }}
                                </span>
                            </template>

                            <!-- Status Column -->
                            <template #item.is_active="{ item }">
                                <v-chip
                                    :color="item.is_active ? 'success' : 'error'"
                                    :text="item.is_active ? 'Active' : 'Inactive'"
                                    size="small"
                                />
                            </template>

                            <!-- Created At Column -->
                            <template #item.created_at="{ item }">
                                <span class="text-body-2">
                                    {{ formatDate(item.created_at) }}
                                </span>
                            </template>

                            <!-- Expires At Column -->
                            <template #item.expires_at="{ item }">
                                <span
                                    v-if="item.expires_at"
                                    class="text-body-2"
                                >
                                    {{ formatDate(item.expires_at) }}
                                </span>
                                <span
                                    v-else
                                    class="text-body-2 text-medium-emphasis"
                                >
                                    Never
                                </span>
                            </template>

                            <!-- Last Used Column -->
                            <template #item.last_used_at="{ item }">
                                <span
                                    v-if="item.last_used_at"
                                    class="text-body-2"
                                >
                                    {{ formatDate(item.last_used_at) }}
                                </span>
                                <span
                                    v-else
                                    class="text-body-2 text-medium-emphasis"
                                >
                                    Never
                                </span>
                            </template>

                            <!-- User UUID Column (Admin Only) -->
                            <template #item.user_uuid="{ item }">
                                <span
                                    v-if="isAdmin"
                                    class="text-body-2 font-mono"
                                >
                                    {{ item.user_uuid }}
                                </span>
                                <span
                                    v-else
                                    class="text-body-2 text-medium-emphasis"
                                >
                                    -
                                </span>
                            </template>

                            <!-- Created By Column (Admin Only) -->
                            <template #item.created_by="{ item }">
                                <span
                                    v-if="isAdmin"
                                    class="text-body-2 font-mono"
                                >
                                    {{ item.created_by }}
                                </span>
                                <span
                                    v-else
                                    class="text-body-2 text-medium-emphasis"
                                >
                                    -
                                </span>
                            </template>

                            <!-- Actions Column -->
                            <template #item.actions="{ item }">
                                <div class="d-flex gap-2">
                                    <v-btn
                                        icon="mdi-eye"
                                        variant="text"
                                        size="small"
                                        color="info"
                                        :disabled="!item.is_active"
                                        @click="viewKey(item)"
                                    />
                                    <v-btn
                                        icon="mdi-delete"
                                        variant="text"
                                        size="small"
                                        color="error"
                                        :disabled="!item.is_active"
                                        @click="confirmRevoke(item)"
                                    />
                                </div>
                            </template>
                        </PaginatedDataTable>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Create API Key Dialog -->
        <v-dialog
            v-model="showCreateDialog"
            max-width="500px"
            @after-enter="focusNameField"
        >
            <v-card>
                <v-card-title>{{ t('api_keys.create.title') }}</v-card-title>
                <v-card-text>
                    <v-form
                        ref="createForm"
                        v-model="createFormValid"
                    >
                        <v-text-field
                            ref="nameField"
                            v-model="createForm.name"
                            :label="t('api_keys.create.name_label')"
                            :rules="[v => !!v || t('api_keys.create.name_required')]"
                            required
                            @input="validateForm"
                        />
                        <v-textarea
                            v-model="createForm.description"
                            :label="t('api_keys.create.description_label')"
                            rows="3"
                        />
                        <v-text-field
                            v-model.number="createForm.expires_in_days"
                            :label="t('api_keys.create.expires_label')"
                            type="number"
                            min="1"
                            max="3650"
                            :hint="t('api_keys.create.expires_hint')"
                        />
                    </v-form>
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn
                        color="grey"
                        variant="text"
                        @click="showCreateDialog = false"
                    >
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn
                        color="primary"
                        :loading="creating"
                        :disabled="!createFormValid"
                        @click="createApiKey"
                    >
                        {{ t('api_keys.create.button') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- View API Key Dialog -->
        <v-dialog
            v-model="showViewDialog"
            max-width="600px"
        >
            <v-card>
                <v-card-title>{{ t('api_keys.view.title') }}</v-card-title>
                <v-card-text>
                    <div v-if="selectedKey">
                        <v-list>
                            <v-list-item>
                                <template #prepend>
                                    <v-icon icon="mdi-key" />
                                </template>
                                <v-list-item-title>{{ t('api_keys.view.name') }}</v-list-item-title>
                                <v-list-item-subtitle>{{ selectedKey.name }}</v-list-item-subtitle>
                            </v-list-item>
                            <v-list-item v-if="selectedKey.description">
                                <template #prepend>
                                    <v-icon icon="mdi-text" />
                                </template>
                                <v-list-item-title>{{
                                    t('api_keys.view.description')
                                }}</v-list-item-title>
                                <v-list-item-subtitle>{{
                                    selectedKey.description
                                }}</v-list-item-subtitle>
                            </v-list-item>
                            <v-list-item>
                                <template #prepend>
                                    <v-icon icon="mdi-calendar" />
                                </template>
                                <v-list-item-title>{{
                                    t('api_keys.view.created')
                                }}</v-list-item-title>
                                <v-list-item-subtitle>{{
                                    formatDate(selectedKey.created_at)
                                }}</v-list-item-subtitle>
                            </v-list-item>
                            <v-list-item v-if="selectedKey.expires_at">
                                <template #prepend>
                                    <v-icon icon="mdi-calendar-clock" />
                                </template>
                                <v-list-item-title>{{
                                    t('api_keys.view.expires')
                                }}</v-list-item-title>
                                <v-list-item-subtitle>{{
                                    formatDate(selectedKey.expires_at)
                                }}</v-list-item-subtitle>
                            </v-list-item>
                            <v-list-item v-if="selectedKey.last_used_at">
                                <template #prepend>
                                    <v-icon icon="mdi-clock" />
                                </template>
                                <v-list-item-title>{{
                                    t('api_keys.view.last_used')
                                }}</v-list-item-title>
                                <v-list-item-subtitle>{{
                                    formatDate(selectedKey.last_used_at)
                                }}</v-list-item-subtitle>
                            </v-list-item>
                        </v-list>
                    </div>
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn
                        color="primary"
                        @click="showViewDialog = false"
                    >
                        {{ t('common.close') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- Created API Key Dialog -->
        <v-dialog
            v-model="showCreatedKeyDialog"
            max-width="600px"
            persistent
        >
            <v-card>
                <v-card-title class="d-flex align-center">
                    <v-icon
                        icon="mdi-check-circle"
                        color="success"
                        class="mr-2"
                    />
                    {{ t('api_keys.created.title') }}
                </v-card-title>
                <v-card-text>
                    <v-alert
                        type="warning"
                        variant="tonal"
                        class="mb-4"
                    >
                        {{ t('api_keys.created.warning') }}
                    </v-alert>

                    <v-text-field
                        id="apiKey"
                        :model-value="createdApiKey"
                        :label="t('api_keys.created.key_label')"
                        readonly
                        variant="outlined"
                        class="mb-4"
                    >
                        <template #append>
                            <v-btn
                                icon="mdi-content-copy"
                                variant="text"
                                size="small"
                                @click="copyApiKey"
                            />
                        </template>
                    </v-text-field>

                    <p class="text-body-2 text-medium-emphasis">
                        {{ t('api_keys.created.description') }}
                    </p>
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn
                        color="primary"
                        @click="closeCreatedKeyDialog"
                    >
                        {{ t('common.close') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- Revoke Confirmation Dialog -->
        <v-dialog
            v-model="showRevokeDialog"
            max-width="400px"
        >
            <v-card>
                <v-card-title>{{ t('api_keys.revoke.title') }}</v-card-title>
                <v-card-text>
                    {{ t('api_keys.revoke.message', { name: keyToRevoke?.name || 'Unknown' }) }}
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn
                        color="grey"
                        variant="text"
                        @click="showRevokeDialog = false"
                    >
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn
                        color="error"
                        :loading="revoking"
                        @click="revokeApiKey"
                    >
                        {{ t('api_keys.revoke.button') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- Success Snackbar -->
        <v-snackbar
            v-model="showSuccessSnackbar"
            color="success"
            timeout="3000"
        >
            {{ successMessage }}
        </v-snackbar>

        <!-- Error Snackbar -->
        <v-snackbar
            v-model="showErrorSnackbar"
            color="error"
            timeout="5000"
        >
            {{ errorMessage }}
        </v-snackbar>
    </v-container>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
    import { useAuthStore } from '@/stores/auth'
    import { typedHttpClient } from '@/api/typed-client'
    import type { ApiKey, CreateApiKeyRequest } from '@/types/schemas'
    import { useTranslations } from '@/composables/useTranslations'
    import { usePagination } from '@/composables/usePagination'
    import PaginatedDataTable from '@/components/tables/PaginatedDataTable.vue'

    const authStore = useAuthStore()
    const { t } = useTranslations()

    // Reactive state
    const loading = ref(false)
    const error = ref('')
    const apiKeys = ref<ApiKey[]>([])
    const showCreateDialog = ref(false)

    // Pagination state with persistence
    const { state: paginationState, setPage, setItemsPerPage } = usePagination('api-keys', 10)
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
    const showViewDialog = ref(false)
    const showRevokeDialog = ref(false)
    const showCreatedKeyDialog = ref(false)
    const showSuccessSnackbar = ref(false)
    const showErrorSnackbar = ref(false)
    const successMessage = ref('')
    const errorMessage = ref('')
    const creating = ref(false)
    const revoking = ref(false)
    const selectedKey = ref<ApiKey | null>(null)
    const keyToRevoke = ref<ApiKey | null>(null)
    const createdApiKey = ref('')
    const createFormValid = ref(false)
    const createForm = ref<CreateApiKeyRequest>({
        name: '',
        description: '',
        expires_in_days: undefined,
    })

    // Component lifecycle flag
    const isComponentMounted = ref(false)

    // Refs for form validation
    const nameField = ref<HTMLInputElement | null>(null)

    // Computed properties
    const isAdmin = computed(() => {
        return authStore.user?.is_admin || false
    })

    const tableHeaders = computed(() => {
        const headers = [
            { title: t('api_keys.table.name'), key: 'name', sortable: true },
            { title: t('api_keys.table.description'), key: 'description', sortable: false },
            { title: t('api_keys.table.status'), key: 'is_active', sortable: true },
            { title: t('api_keys.table.created'), key: 'created_at', sortable: true },
            { title: t('api_keys.table.expires'), key: 'expires_at', sortable: true },
            { title: t('api_keys.table.last_used'), key: 'last_used_at', sortable: true },
        ]

        // Add admin-only columns
        if (isAdmin.value) {
            headers.splice(3, 0, {
                title: t('api_keys.table.user_id'),
                key: 'user_uuid',
                sortable: false,
            })
            headers.splice(4, 0, {
                title: t('api_keys.table.created_by'),
                key: 'created_by',
                sortable: false,
            })
        }

        headers.push({ title: t('api_keys.table.actions'), key: 'actions', sortable: false })

        return headers
    })

    // Methods
    const loadApiKeys = async (page = 1, itemsPerPage = 10) => {
        // Don't load if component is not mounted or user is not authenticated
        if (!isComponentMounted.value || !authStore.isAuthenticated) {
            return
        }

        loading.value = true
        error.value = ''

        try {
            console.log(`Loading API keys: page=${page}, itemsPerPage=${itemsPerPage}`)
            const response = await typedHttpClient.getApiKeys(page, itemsPerPage)
            apiKeys.value = response.data
            if (response.meta?.pagination) {
                totalItems.value = response.meta.pagination.total
                totalPages.value = response.meta.pagination.total_pages
                paginationMeta.value = response.meta.pagination
            } else {
                totalItems.value = apiKeys.value.length
                totalPages.value = 1
                paginationMeta.value = null
            }
        } catch (err) {
            console.error('Failed to load API keys:', err)
            error.value = err instanceof Error ? err.message : 'Failed to load API keys'
        } finally {
            loading.value = false
        }
    }

    const handlePageChange = (page: number) => {
        console.log('Page changed to:', page)
        currentPage.value = page
        setPage(page)
        loadApiKeys(currentPage.value, itemsPerPage.value)
    }

    const handleItemsPerPageChange = (newItemsPerPage: number) => {
        console.log('Items per page changed to:', newItemsPerPage)
        itemsPerPage.value = newItemsPerPage
        setItemsPerPage(newItemsPerPage)
        loadApiKeys(currentPage.value, itemsPerPage.value)
    }

    const createApiKey = async () => {
        if (!createFormValid.value) {
            return
        }

        creating.value = true

        try {
            // Create a clean object without circular references
            const requestData: CreateApiKeyRequest = {
                name: createForm.value.name,
                description: createForm.value.description || undefined,
                expires_in_days: createForm.value.expires_in_days || undefined,
            }

            const result = await typedHttpClient.createApiKey(requestData)

            // Show the API key in the dedicated dialog
            createdApiKey.value = result.api_key
            showCreatedKeyDialog.value = true

            // Reset form and close dialog
            createForm.value = { name: '', description: '', expires_in_days: undefined }
            showCreateDialog.value = false

            // Reload the list
            await loadApiKeys()
        } catch (err) {
            showErrorSnackbar.value = true
            errorMessage.value = err instanceof Error ? err.message : t('api_keys.create.error')
        } finally {
            creating.value = false
        }
    }

    const viewKey = (key: ApiKey) => {
        selectedKey.value = key
        showViewDialog.value = true
    }

    const confirmRevoke = (key: ApiKey) => {
        keyToRevoke.value = key
        showRevokeDialog.value = true
    }

    const revokeApiKey = async () => {
        if (!keyToRevoke.value) {
            return
        }

        revoking.value = true

        try {
            await typedHttpClient.revokeApiKey(keyToRevoke.value.uuid)

            showSuccessSnackbar.value = true
            successMessage.value = t('api_keys.revoke.success')

            showRevokeDialog.value = false
            keyToRevoke.value = null

            // Reload the list
            await loadApiKeys()
        } catch (err) {
            showErrorSnackbar.value = true
            errorMessage.value = err instanceof Error ? err.message : t('api_keys.revoke.error')
        } finally {
            revoking.value = false
        }
    }

    const formatDate = (dateString: string | null): string => {
        if (!dateString) {
            return 'Never'
        }
        return new Date(dateString).toLocaleString()
    }

    const validateForm = () => {
        createFormValid.value = !!createForm.value.name.trim()
    }

    const focusNameField = () => {
        // Use nextTick to ensure the field is rendered
        nextTick(() => {
            if (nameField.value) {
                nameField.value.focus()
            }
        })
    }

    const copyApiKey = () => {
        const apiKey = createdApiKey.value

        // Modern approach
        if (navigator && navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard
                .writeText(apiKey)
                .then(() => {
                    console.log('API key copied to clipboard via modern method:', apiKey)
                    showSuccessSnackbar.value = true
                    successMessage.value = t('api_keys.created.copied')
                })
                .catch(err => {
                    console.error('Failed to copy API key:', err)
                })
        } else {
            // Fallback method (old browsers / JS) - based on masteringjs.io tutorial
            const input = document.querySelector('#apiKey')
            input.select()
            input.setSelectionRange(0, 99999)
            try {
                document.execCommand('copy')
                console.log('API key copied to clipboard via old method:', apiKey)
                showSuccessSnackbar.value = true
                successMessage.value = t('api_keys.created.copied')
            } catch (err) {
                console.error('Failed to copy API key:', err)
            }
            document.body.removeChild(textArea)
        }
    }

    const closeCreatedKeyDialog = () => {
        showCreatedKeyDialog.value = false
        createdApiKey.value = ''
    }

    // Lifecycle
    onMounted(() => {
        isComponentMounted.value = true
        loadApiKeys(currentPage.value, itemsPerPage.value)
    })

    onUnmounted(() => {
        isComponentMounted.value = false
    })
</script>
