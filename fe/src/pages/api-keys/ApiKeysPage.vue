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
                            Create New Key
                        </v-btn>
                    </v-card-title>

                    <v-card-text>
                        <!-- Loading State -->
                        <v-row v-if="loading">
                            <v-col
                                cols="12"
                                class="text-center"
                            >
                                <v-progress-circular
                                    indeterminate
                                    color="primary"
                                    size="64"
                                />
                                <div class="mt-4">Loading API keys...</div>
                            </v-col>
                        </v-row>

                        <!-- Error State -->
                        <v-alert
                            v-else-if="error"
                            type="error"
                            variant="tonal"
                            class="mb-4"
                        >
                            {{ error }}
                        </v-alert>

                        <!-- API Keys Table -->
                        <v-data-table
                            v-else
                            :headers="tableHeaders"
                            :items="apiKeys"
                            :loading="loading"
                            :items-per-page="10"
                            class="elevation-1"
                            responsive
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
                                        icon="mdi-pencil"
                                        variant="text"
                                        size="small"
                                        color="warning"
                                        :disabled="!item.is_active"
                                        @click="editKey(item)"
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
                        </v-data-table>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Create API Key Dialog -->
        <v-dialog
            v-model="showCreateDialog"
            max-width="500px"
        >
            <v-card>
                <v-card-title>Create New API Key</v-card-title>
                <v-card-text>
                    <v-form
                        ref="createForm"
                        v-model="createFormValid"
                    >
                        <v-text-field
                            v-model="createForm.name"
                            label="Key Name"
                            :rules="[v => !!v || 'Name is required']"
                            required
                        />
                        <v-textarea
                            v-model="createForm.description"
                            label="Description (Optional)"
                            rows="3"
                        />
                        <v-text-field
                            v-model.number="createForm.expires_in_days"
                            label="Expires in Days (Optional)"
                            type="number"
                            min="1"
                            max="3650"
                            hint="Leave empty for no expiration"
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
                        Cancel
                    </v-btn>
                    <v-btn
                        color="primary"
                        :loading="creating"
                        :disabled="!createFormValid"
                        @click="createApiKey"
                    >
                        Create
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
                <v-card-title>API Key Details</v-card-title>
                <v-card-text>
                    <div v-if="selectedKey">
                        <v-list>
                            <v-list-item>
                                <template #prepend>
                                    <v-icon icon="mdi-key" />
                                </template>
                                <v-list-item-title>Name</v-list-item-title>
                                <v-list-item-subtitle>{{ selectedKey.name }}</v-list-item-subtitle>
                            </v-list-item>
                            <v-list-item v-if="selectedKey.description">
                                <template #prepend>
                                    <v-icon icon="mdi-text" />
                                </template>
                                <v-list-item-title>Description</v-list-item-title>
                                <v-list-item-subtitle>{{
                                    selectedKey.description
                                }}</v-list-item-subtitle>
                            </v-list-item>
                            <v-list-item>
                                <template #prepend>
                                    <v-icon icon="mdi-calendar" />
                                </template>
                                <v-list-item-title>Created</v-list-item-title>
                                <v-list-item-subtitle>{{
                                    formatDate(selectedKey.created_at)
                                }}</v-list-item-subtitle>
                            </v-list-item>
                            <v-list-item v-if="selectedKey.expires_at">
                                <template #prepend>
                                    <v-icon icon="mdi-calendar-clock" />
                                </template>
                                <v-list-item-title>Expires</v-list-item-title>
                                <v-list-item-subtitle>{{
                                    formatDate(selectedKey.expires_at)
                                }}</v-list-item-subtitle>
                            </v-list-item>
                            <v-list-item v-if="selectedKey.last_used_at">
                                <template #prepend>
                                    <v-icon icon="mdi-clock" />
                                </template>
                                <v-list-item-title>Last Used</v-list-item-title>
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
                        Close
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
                <v-card-title>Confirm Revoke</v-card-title>
                <v-card-text>
                    Are you sure you want to revoke the API key "{{ keyToRevoke?.name }}"? This
                    action cannot be undone.
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn
                        color="grey"
                        variant="text"
                        @click="showRevokeDialog = false"
                    >
                        Cancel
                    </v-btn>
                    <v-btn
                        color="error"
                        :loading="revoking"
                        @click="revokeApiKey"
                    >
                        Revoke
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
    import { ref, computed, onMounted, onUnmounted } from 'vue'
    import { useAuthStore } from '@/stores/auth'
    import { typedHttpClient } from '@/api/typed-client'
    import type { ApiKey, CreateApiKeyRequest } from '@/types/schemas'

    const authStore = useAuthStore()

    // Reactive state
    const loading = ref(false)
    const error = ref('')
    const apiKeys = ref<ApiKey[]>([])
    const showCreateDialog = ref(false)
    const showViewDialog = ref(false)
    const showRevokeDialog = ref(false)
    const showSuccessSnackbar = ref(false)
    const showErrorSnackbar = ref(false)
    const successMessage = ref('')
    const errorMessage = ref('')
    const creating = ref(false)
    const revoking = ref(false)
    const selectedKey = ref<ApiKey | null>(null)
    const keyToRevoke = ref<ApiKey | null>(null)
    const createFormValid = ref(false)
    const createForm = ref<CreateApiKeyRequest>({
        name: '',
        description: '',
        expires_in_days: undefined,
    })

    // Component lifecycle flag
    const isComponentMounted = ref(false)

    // Computed properties
    const isAdmin = computed(() => {
        return authStore.user?.is_admin || false
    })

    const tableHeaders = computed(() => {
        const headers = [
            { title: 'Name', key: 'name', sortable: true },
            { title: 'Description', key: 'description', sortable: false },
            { title: 'Status', key: 'is_active', sortable: true },
            { title: 'Created', key: 'created_at', sortable: true },
            { title: 'Expires', key: 'expires_at', sortable: true },
            { title: 'Last Used', key: 'last_used_at', sortable: true },
        ]

        // Add admin-only columns
        if (isAdmin.value) {
            headers.splice(3, 0, { title: 'User ID', key: 'user_uuid', sortable: false })
            headers.splice(4, 0, { title: 'Created By', key: 'created_by', sortable: false })
        }

        headers.push({ title: 'Actions', key: 'actions', sortable: false })

        return headers
    })

    // Methods
    const loadApiKeys = async () => {
        // Don't load if component is not mounted or user is not authenticated
        if (!isComponentMounted.value || !authStore.isAuthenticated) {
            return
        }

        loading.value = true
        error.value = ''

        try {
            apiKeys.value = await typedHttpClient.getApiKeys()
        } catch (err) {
            error.value = err instanceof Error ? err.message : 'Failed to load API keys'
        } finally {
            loading.value = false
        }
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

            // Show the API key to the user (only shown once)
            showSuccessSnackbar.value = true
            successMessage.value = `API key created successfully! Key: ${result.api_key}`

            // Reset form and close dialog
            createForm.value = { name: '', description: '', expires_in_days: undefined }
            showCreateDialog.value = false

            // Reload the list
            await loadApiKeys()
        } catch (err) {
            showErrorSnackbar.value = true
            errorMessage.value = err instanceof Error ? err.message : 'Failed to create API key'
        } finally {
            creating.value = false
        }
    }

    const viewKey = (key: ApiKey) => {
        selectedKey.value = key
        showViewDialog.value = true
    }

    const editKey = (key: ApiKey) => {
        // TODO: Implement edit functionality
        console.log('Edit key:', key)
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
            successMessage.value = 'API key revoked successfully'

            showRevokeDialog.value = false
            keyToRevoke.value = null

            // Reload the list
            await loadApiKeys()
        } catch (err) {
            showErrorSnackbar.value = true
            errorMessage.value = err instanceof Error ? err.message : 'Failed to revoke API key'
        } finally {
            revoking.value = false
        }
    }

    const formatDate = (dateString: string) => {
        return new Date(dateString).toLocaleString()
    }

    // Lifecycle
    onMounted(() => {
        isComponentMounted.value = true
        loadApiKeys()
    })

    onUnmounted(() => {
        isComponentMounted.value = false
    })
</script>
