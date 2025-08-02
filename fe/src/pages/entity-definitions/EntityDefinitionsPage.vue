<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title class="d-flex align-center justify-space-between pa-4">
                        <div class="d-flex align-center">
                            <v-icon
                                icon="mdi-file-tree"
                                class="mr-3"
                            />
                            <span class="text-h4">{{ t('navigation.entity_definitions') }}</span>
                        </div>
                        <v-btn
                            color="primary"
                            prepend-icon="mdi-plus"
                            @click="showCreateDialog = true"
                        >
                            {{ t('entity_definitions.create.button') }}
                        </v-btn>
                    </v-card-title>

                    <v-card-text>
                        <v-row>
                            <!-- Tree View -->
                            <v-col cols="4">
                                <v-card variant="outlined">
                                    <v-card-title class="text-h6 pa-3">
                                        <v-icon
                                            icon="mdi-folder-tree"
                                            class="mr-2"
                                        />
                                        {{ t('entity_definitions.table.display_name') }}
                                    </v-card-title>
                                    <v-card-text class="pa-0">
                                        <v-treeview
                                            v-model="selectedItems"
                                            :items="treeItems"
                                            :loading="loading"
                                            item-key="uuid"
                                            activatable
                                            hoverable
                                            @update:active="handleTreeSelection"
                                        >
                                            <template #prepend="{ item }">
                                                <v-icon
                                                    :icon="item.icon || 'mdi-file-document'"
                                                    :color="item.published ? 'success' : 'grey'"
                                                />
                                            </template>
                                        </v-treeview>
                                    </v-card-text>
                                </v-card>
                            </v-col>

                            <!-- Details Panel -->
                            <v-col cols="8">
                                <v-card
                                    v-if="selectedDefinition"
                                    variant="outlined"
                                >
                                    <v-card-title
                                        class="d-flex align-center justify-space-between pa-4"
                                    >
                                        <div class="d-flex align-center">
                                            <v-icon
                                                :icon="
                                                    selectedDefinition.icon || 'mdi-file-document'
                                                "
                                                class="mr-3"
                                            />
                                            <span class="text-h5">{{
                                                selectedDefinition.display_name
                                            }}</span>
                                        </div>
                                        <div>
                                            <v-btn
                                                color="primary"
                                                variant="outlined"
                                                prepend-icon="mdi-pencil"
                                                class="mr-2"
                                                @click="editDefinition"
                                            >
                                                Edit
                                            </v-btn>
                                            <v-btn
                                                color="success"
                                                variant="outlined"
                                                prepend-icon="mdi-database"
                                                :loading="applyingSchema"
                                                class="mr-2"
                                                @click="applySchema"
                                            >
                                                {{ t('entity_definitions.details.apply_schema') }}
                                            </v-btn>
                                            <v-btn
                                                color="error"
                                                variant="outlined"
                                                prepend-icon="mdi-delete"
                                                @click="showDeleteDialog = true"
                                            >
                                                Delete
                                            </v-btn>
                                        </div>
                                    </v-card-title>

                                    <v-card-text>
                                        <v-tabs v-model="activeTab">
                                            <v-tab value="meta">{{
                                                t('entity_definitions.details.meta_info')
                                            }}</v-tab>
                                            <v-tab value="fields">{{
                                                t('entity_definitions.details.fields')
                                            }}</v-tab>
                                        </v-tabs>

                                        <v-window v-model="activeTab">
                                            <!-- Meta Information Tab -->
                                            <v-window-item value="meta">
                                                <v-row class="mt-4">
                                                    <v-col cols="6">
                                                        <v-list>
                                                            <v-list-item>
                                                                <template #prepend>
                                                                    <v-icon icon="mdi-tag" />
                                                                </template>
                                                                <v-list-item-title
                                                                >Entity Type</v-list-item-title
                                                                >
                                                                <v-list-item-subtitle>{{
                                                                    selectedDefinition.entity_type
                                                                }}</v-list-item-subtitle>
                                                            </v-list-item>
                                                            <v-list-item>
                                                                <template #prepend>
                                                                    <v-icon icon="mdi-text" />
                                                                </template>
                                                                <v-list-item-title
                                                                >Display Name</v-list-item-title
                                                                >
                                                                <v-list-item-subtitle>{{
                                                                    selectedDefinition.display_name
                                                                }}</v-list-item-subtitle>
                                                            </v-list-item>
                                                            <v-list-item
                                                                v-if="
                                                                    selectedDefinition.description
                                                                "
                                                            >
                                                                <template #prepend>
                                                                    <v-icon
                                                                        icon="mdi-information"
                                                                    />
                                                                </template>
                                                                <v-list-item-title
                                                                >Description</v-list-item-title
                                                                >
                                                                <v-list-item-subtitle>{{
                                                                    selectedDefinition.description
                                                                }}</v-list-item-subtitle>
                                                            </v-list-item>
                                                            <v-list-item
                                                                v-if="selectedDefinition.group_name"
                                                            >
                                                                <template #prepend>
                                                                    <v-icon icon="mdi-folder" />
                                                                </template>
                                                                <v-list-item-title
                                                                >Group</v-list-item-title
                                                                >
                                                                <v-list-item-subtitle>{{
                                                                    selectedDefinition.group_name
                                                                }}</v-list-item-subtitle>
                                                            </v-list-item>
                                                        </v-list>
                                                    </v-col>
                                                    <v-col cols="6">
                                                        <v-list>
                                                            <v-list-item>
                                                                <template #prepend>
                                                                    <v-icon icon="mdi-calendar" />
                                                                </template>
                                                                <v-list-item-title
                                                                >Created</v-list-item-title
                                                                >
                                                                <v-list-item-subtitle>{{
                                                                    formatDate(
                                                                        selectedDefinition.created_at
                                                                    )
                                                                }}</v-list-item-subtitle>
                                                            </v-list-item>
                                                            <v-list-item>
                                                                <template #prepend>
                                                                    <v-icon
                                                                        icon="mdi-calendar-edit"
                                                                    />
                                                                </template>
                                                                <v-list-item-title
                                                                >Updated</v-list-item-title
                                                                >
                                                                <v-list-item-subtitle>{{
                                                                    formatDate(
                                                                        selectedDefinition.updated_at
                                                                    )
                                                                }}</v-list-item-subtitle>
                                                            </v-list-item>
                                                            <v-list-item>
                                                                <template #prepend>
                                                                    <v-icon icon="mdi-counter" />
                                                                </template>
                                                                <v-list-item-title>
                                                                    Version
                                                                </v-list-item-title>
                                                                <v-list-item-subtitle>{{
                                                                    selectedDefinition.version
                                                                }}</v-list-item-subtitle>
                                                            </v-list-item>
                                                            <v-list-item>
                                                                <template #prepend>
                                                                    <v-icon
                                                                        icon="mdi-checkbox-marked-circle"
                                                                    />
                                                                </template>
                                                                <v-list-item-title>
                                                                    Status
                                                                </v-list-item-title>
                                                                <v-list-item-subtitle>
                                                                    <v-chip
                                                                        :color="
                                                                            selectedDefinition.published
                                                                                ? 'success'
                                                                                : 'warning'
                                                                        "
                                                                        size="small"
                                                                    >
                                                                        {{
                                                                            selectedDefinition.published
                                                                                ? 'Published'
                                                                                : 'Draft'
                                                                        }}
                                                                    </v-chip>
                                                                </v-list-item-subtitle>
                                                            </v-list-item>
                                                        </v-list>
                                                    </v-col>
                                                </v-row>
                                            </v-window-item>

                                            <!-- Fields Tab -->
                                            <v-window-item value="fields">
                                                <div class="mt-4">
                                                    <div
                                                        class="d-flex align-center justify-space-between mb-4"
                                                    >
                                                        <h3 class="text-h6">
                                                            {{
                                                                t('entity_definitions.fields.title')
                                                            }}
                                                        </h3>
                                                        <v-btn
                                                            color="primary"
                                                            prepend-icon="mdi-plus"
                                                            @click="addField"
                                                        >
                                                            {{
                                                                t(
                                                                    'entity_definitions.fields.add_field'
                                                                )
                                                            }}
                                                        </v-btn>
                                                    </div>

                                                    <v-data-table
                                                        :headers="fieldHeaders"
                                                        :items="selectedDefinition.fields"
                                                        :loading="loading"
                                                        class="elevation-1"
                                                    >
                                                        <template #item.field_type="{ item }">
                                                            <v-chip
                                                                size="small"
                                                                color="primary"
                                                            >
                                                                {{ item.field_type }}
                                                            </v-chip>
                                                        </template>
                                                        <template #item.required="{ item }">
                                                            <v-icon
                                                                :icon="
                                                                    item.required
                                                                        ? 'mdi-check'
                                                                        : 'mdi-close'
                                                                "
                                                                :color="
                                                                    item.required
                                                                        ? 'success'
                                                                        : 'grey'
                                                                "
                                                            />
                                                        </template>
                                                        <template #item.indexed="{ item }">
                                                            <v-icon
                                                                :icon="
                                                                    item.indexed
                                                                        ? 'mdi-check'
                                                                        : 'mdi-close'
                                                                "
                                                                :color="
                                                                    item.indexed
                                                                        ? 'success'
                                                                        : 'grey'
                                                                "
                                                            />
                                                        </template>
                                                        <template #item.filterable="{ item }">
                                                            <v-icon
                                                                :icon="
                                                                    item.filterable
                                                                        ? 'mdi-check'
                                                                        : 'mdi-close'
                                                                "
                                                                :color="
                                                                    item.filterable
                                                                        ? 'success'
                                                                        : 'grey'
                                                                "
                                                            />
                                                        </template>
                                                        <template #item.actions="{ item }">
                                                            <v-btn
                                                                icon="mdi-pencil"
                                                                size="small"
                                                                variant="text"
                                                                @click="editField(item)"
                                                            />
                                                            <v-btn
                                                                icon="mdi-delete"
                                                                size="small"
                                                                variant="text"
                                                                color="error"
                                                                @click="removeField(item)"
                                                            />
                                                        </template>
                                                    </v-data-table>
                                                </div>
                                            </v-window-item>
                                        </v-window>
                                    </v-card-text>
                                </v-card>

                                <v-card
                                    v-else
                                    variant="outlined"
                                >
                                    <v-card-text class="text-center pa-8">
                                        <v-icon
                                            icon="mdi-file-document-outline"
                                            size="64"
                                            color="grey"
                                            class="mb-4"
                                        />
                                                                                 <h3 class="text-h6 text-grey">
                                             {{ t('entity_definitions.details.select_entity') }}
                                         </h3>
                                         <p class="text-body-2 text-grey">
                                             {{ t('entity_definitions.details.select_entity_description') }}
                                         </p>
                                    </v-card-text>
                                </v-card>
                            </v-col>
                        </v-row>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Create Entity Definition Dialog -->
        <v-dialog
            v-model="showCreateDialog"
            max-width="800px"
        >
            <v-card>
                <v-card-title class="text-h5 pa-4">
                    {{ t('entity_definitions.create.title') }}
                </v-card-title>
                <v-card-text>
                    <v-form
                        ref="createFormRef"
                        v-model="createFormValid"
                    >
                        <v-row>
                            <v-col cols="6">
                                <v-text-field
                                    v-model="createForm.entity_type"
                                    :label="t('entity_definitions.create.entity_type_label')"
                                    :hint="t('entity_definitions.create.entity_type_hint')"
                                    :rules="[
                                        v =>
                                            !!v ||
                                            t('entity_definitions.create.entity_type_required'),
                                    ]"
                                    required
                                />
                            </v-col>
                            <v-col cols="6">
                                <v-text-field
                                    v-model="createForm.display_name"
                                    :label="t('entity_definitions.create.display_name_label')"
                                    :rules="[
                                        v =>
                                            !!v ||
                                            t('entity_definitions.create.display_name_required'),
                                    ]"
                                    required
                                />
                            </v-col>
                        </v-row>
                        <v-row>
                            <v-col cols="12">
                                <v-textarea
                                    v-model="createForm.description"
                                    :label="t('entity_definitions.create.description_label')"
                                    rows="3"
                                />
                            </v-col>
                        </v-row>
                        <v-row>
                            <v-col cols="6">
                                <v-text-field
                                    v-model="createForm.group_name"
                                    :label="t('entity_definitions.create.group_name_label')"
                                />
                            </v-col>
                            <v-col cols="6">
                                <v-text-field
                                    v-model="createForm.icon"
                                    :label="t('entity_definitions.create.icon_label')"
                                />
                            </v-col>
                        </v-row>
                        <v-row>
                            <v-col cols="6">
                                <v-switch
                                    v-model="createForm.allow_children"
                                    :label="t('entity_definitions.create.allow_children_label')"
                                />
                            </v-col>
                            <v-col cols="6">
                                <v-switch
                                    v-model="createForm.published"
                                    :label="t('entity_definitions.create.published_label')"
                                />
                            </v-col>
                        </v-row>
                    </v-form>
                </v-card-text>
                <v-card-actions class="pa-4">
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
                        @click="createEntityDefinition"
                    >
                        Create
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- Edit Entity Definition Dialog -->
        <v-dialog
            v-model="showEditDialog"
            max-width="1200px"
        >
            <v-card>
                <v-card-title class="text-h5 pa-4">
                    {{ t('entity_definitions.edit.title') }}
                </v-card-title>
                <v-card-text>
                    <v-form
                        ref="editFormRef"
                        v-model="editFormValid"
                    >
                        <v-row>
                            <v-col cols="6">
                                <v-text-field
                                    v-model="editForm.entity_type"
                                    :label="t('entity_definitions.create.entity_type_label')"
                                    :hint="t('entity_definitions.create.entity_type_hint')"
                                    :rules="[
                                        v =>
                                            !!v ||
                                            t('entity_definitions.create.entity_type_required'),
                                    ]"
                                    required
                                    readonly
                                />
                            </v-col>
                            <v-col cols="6">
                                <v-text-field
                                    v-model="editForm.display_name"
                                    :label="t('entity_definitions.create.display_name_label')"
                                    :rules="[
                                        v =>
                                            !!v ||
                                            t('entity_definitions.create.display_name_required'),
                                    ]"
                                    required
                                />
                            </v-col>
                        </v-row>
                        <v-row>
                            <v-col cols="12">
                                <v-textarea
                                    v-model="editForm.description"
                                    :label="t('entity_definitions.create.description_label')"
                                    rows="3"
                                />
                            </v-col>
                        </v-row>
                        <v-row>
                            <v-col cols="6">
                                <v-text-field
                                    v-model="editForm.group_name"
                                    :label="t('entity_definitions.create.group_name_label')"
                                />
                            </v-col>
                            <v-col cols="6">
                                <v-text-field
                                    v-model="editForm.icon"
                                    :label="t('entity_definitions.create.icon_label')"
                                />
                            </v-col>
                        </v-row>
                        <v-row>
                            <v-col cols="6">
                                <v-switch
                                    v-model="editForm.allow_children"
                                    :label="t('entity_definitions.create.allow_children_label')"
                                />
                            </v-col>
                            <v-col cols="6">
                                <v-switch
                                    v-model="editForm.published"
                                    :label="t('entity_definitions.create.published_label')"
                                />
                            </v-col>
                        </v-row>

                        <!-- Fields Section -->
                        <v-divider class="my-4" />
                        <div class="d-flex align-center justify-space-between mb-4">
                            <h3 class="text-h6">{{ t('entity_definitions.fields.title') }}</h3>
                            <v-btn
                                color="primary"
                                prepend-icon="mdi-plus"
                                @click="addFieldToEdit"
                            >
                                {{ t('entity_definitions.fields.add_field') }}
                            </v-btn>
                        </div>

                        <v-data-table
                            :headers="fieldHeaders"
                            :items="editForm.fields"
                            class="elevation-1"
                        >
                            <template #item.field_type="{ item }">
                                <v-chip
                                    size="small"
                                    color="primary"
                                >
                                    {{ item.field_type }}
                                </v-chip>
                            </template>
                            <template #item.required="{ item }">
                                <v-icon
                                    :icon="item.required ? 'mdi-check' : 'mdi-close'"
                                    :color="item.required ? 'success' : 'grey'"
                                />
                            </template>
                            <template #item.indexed="{ item }">
                                <v-icon
                                    :icon="item.indexed ? 'mdi-check' : 'mdi-close'"
                                    :color="item.indexed ? 'success' : 'grey'"
                                />
                            </template>
                            <template #item.filterable="{ item }">
                                <v-icon
                                    :icon="item.filterable ? 'mdi-check' : 'mdi-close'"
                                    :color="item.filterable ? 'success' : 'grey'"
                                />
                            </template>
                            <template #item.actions="{ item }">
                                <v-btn
                                    icon="mdi-pencil"
                                    size="small"
                                    variant="text"
                                    @click="editFieldInEdit(item)"
                                />
                                <v-btn
                                    icon="mdi-delete"
                                    size="small"
                                    variant="text"
                                    color="error"
                                    @click="removeFieldFromEdit(item)"
                                />
                            </template>
                        </v-data-table>
                    </v-form>
                </v-card-text>
                <v-card-actions class="pa-4">
                    <v-spacer />
                    <v-btn
                        color="grey"
                        variant="text"
                        @click="showEditDialog = false"
                    >
                        {{ t('entity_definitions.edit.cancel') }}
                    </v-btn>
                    <v-btn
                        color="primary"
                        :loading="updating"
                        :disabled="!editFormValid"
                        @click="updateEntityDefinition"
                    >
                        {{ t('entity_definitions.edit.save') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- Delete Confirmation Dialog -->
        <v-dialog
            v-model="showDeleteDialog"
            max-width="500px"
        >
            <v-card>
                <v-card-title class="text-h5 pa-4">
                    {{ t('entity_definitions.delete.title') }}
                </v-card-title>
                <v-card-text>
                    <p>
                        {{
                            t('entity_definitions.delete.message', {
                                name: selectedDefinition?.display_name || 'Unknown',
                            })
                        }}
                    </p>
                </v-card-text>
                <v-card-actions class="pa-4">
                    <v-spacer />
                    <v-btn
                        color="grey"
                        variant="text"
                        @click="showDeleteDialog = false"
                    >
                        {{ t('common.cancel') }}
                    </v-btn>
                    <v-btn
                        color="error"
                        :loading="deleting"
                        @click="deleteEntityDefinition"
                    >
                        {{ t('entity_definitions.delete.button') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <!-- Snackbars -->
        <v-snackbar
            v-model="showSuccessSnackbar"
            color="success"
        >
            {{ successMessage }}
        </v-snackbar>
        <v-snackbar
            v-model="showErrorSnackbar"
            color="error"
        >
            {{ errorMessage }}
        </v-snackbar>
    </v-container>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted, onUnmounted } from 'vue'
    import { useAuthStore } from '@/stores/auth'
    import { typedHttpClient } from '@/api/typed-client'
    import type {
        EntityDefinition,
        CreateEntityDefinitionRequest,
        UpdateEntityDefinitionRequest,
        FieldDefinition,
    } from '@/types/schemas'
    import { useTranslations } from '@/composables/useTranslations'

    const authStore = useAuthStore()
    const { t } = useTranslations()

    // Reactive state
    const loading = ref(false)
    const error = ref('')
    const entityDefinitions = ref<EntityDefinition[]>([])
    const selectedDefinition = ref<EntityDefinition | null>(null)
    const selectedItems = ref<string[]>([])
    const activeTab = ref('meta')
    const applyingSchema = ref(false)

    // Dialog states
    const showCreateDialog = ref(false)
    const showEditDialog = ref(false)
    const showDeleteDialog = ref(false)
    const showSuccessSnackbar = ref(false)
    const showErrorSnackbar = ref(false)
    const successMessage = ref('')
    const errorMessage = ref('')

    // Form states
    const creating = ref(false)
    const updating = ref(false)
    const deleting = ref(false)
    const createFormValid = ref(false)
    const editFormValid = ref(false)

    // Form refs
    const createFormRef = ref<HTMLFormElement | null>(null)
    const editFormRef = ref<HTMLFormElement | null>(null)

    // Component lifecycle flag
    const isComponentMounted = ref(false)

    // Create form
    const createForm = ref<CreateEntityDefinitionRequest>({
        entity_type: '',
        display_name: '',
        description: '',
        group_name: '',
        allow_children: false,
        icon: '',
        fields: [],
        published: false,
    })

    // Edit form
    const editForm = ref<UpdateEntityDefinitionRequest>({
        entity_type: '',
        display_name: '',
        description: '',
        group_name: '',
        allow_children: false,
        icon: '',
        fields: [],
        published: false,
    })

    // Computed properties
    const treeItems = computed(() => {
        return entityDefinitions.value.map(def => ({
            uuid: def.uuid,
            display_name: def.display_name,
            entity_type: def.entity_type,
            icon: def.icon || 'mdi-file-document',
            published: def.published,
            children: [],
        }))
    })

    const fieldHeaders = computed(() => [
        { title: t('entity_definitions.fields.field_name'), key: 'name', sortable: true },
        { title: t('entity_definitions.fields.display_name'), key: 'display_name', sortable: true },
        { title: t('entity_definitions.fields.field_type'), key: 'field_type', sortable: true },
        {
            title: t('entity_definitions.fields.required'),
            key: 'required',
            sortable: true,
            align: 'center' as const,
        },
        {
            title: t('entity_definitions.fields.indexed'),
            key: 'indexed',
            sortable: true,
            align: 'center' as const,
        },
        {
            title: t('entity_definitions.fields.filterable'),
            key: 'filterable',
            sortable: true,
            align: 'center' as const,
        },
        { title: t('entity_definitions.fields.description'), key: 'description', sortable: true },
        {
            title: t('entity_definitions.table.actions'),
            key: 'actions',
            sortable: false,
            align: 'center' as const,
        },
    ])

    // Methods
    const loadEntityDefinitions = async () => {
        if (!isComponentMounted.value || !authStore.isAuthenticated) {
            return
        }

        loading.value = true
        error.value = ''

        try {
            const response = await typedHttpClient.getEntityDefinitions()
            entityDefinitions.value = response.data || []
        } catch (err) {
            console.error('Failed to load entity definitions:', err)
            error.value = err instanceof Error ? err.message : 'Failed to load entity definitions'
            entityDefinitions.value = []
        } finally {
            loading.value = false
        }
    }

    const handleTreeSelection = (items: string[]) => {
        console.log('Tree selection changed:', items)
        if (items.length > 0) {
            const selectedUuid = items[0]
            console.log('Selected UUID:', selectedUuid)
            const definition = entityDefinitions.value.find(d => d.uuid === selectedUuid)
            console.log('Found definition:', definition)
            if (definition) {
                selectedDefinition.value = definition
            }
        } else {
            selectedDefinition.value = null
        }
    }

    const createEntityDefinition = async () => {
        if (!createFormValid.value) {
            return
        }

        creating.value = true

        try {
            await typedHttpClient.createEntityDefinition(createForm.value)

            // Reset form and close dialog
            createForm.value = {
                entity_type: '',
                display_name: '',
                description: '',
                group_name: '',
                allow_children: false,
                icon: '',
                fields: [],
                published: false,
            }
            showCreateDialog.value = false

            // Reload the list
            await loadEntityDefinitions()

            showSuccessSnackbar.value = true
            successMessage.value = 'Entity definition created successfully'
        } catch (err) {
            showErrorSnackbar.value = true
            errorMessage.value =
                err instanceof Error ? err.message : t('entity_definitions.create.error')
        } finally {
            creating.value = false
        }
    }

    const editDefinition = () => {
        if (!selectedDefinition.value) {
            return
        }

        editForm.value = {
            entity_type: selectedDefinition.value.entity_type,
            display_name: selectedDefinition.value.display_name,
            description: selectedDefinition.value.description || '',
            group_name: selectedDefinition.value.group_name || '',
            allow_children: selectedDefinition.value.allow_children,
            icon: selectedDefinition.value.icon || '',
            fields: [...selectedDefinition.value.fields],
            published: selectedDefinition.value.published || false,
        }
        showEditDialog.value = true
    }

    const updateEntityDefinition = async () => {
        if (!editFormValid.value || !selectedDefinition.value) {
            return
        }

        updating.value = true

        try {
            await typedHttpClient.updateEntityDefinition(
                selectedDefinition.value.uuid!,
                editForm.value
            )

            // Update the selected definition
            selectedDefinition.value = {
                ...selectedDefinition.value,
                ...editForm.value,
            }

            // Update the list
            const index = entityDefinitions.value.findIndex(
                d => d.uuid === selectedDefinition.value?.uuid
            )
            if (index !== -1) {
                entityDefinitions.value[index] = selectedDefinition.value
            }

            showEditDialog.value = false

            showSuccessSnackbar.value = true
            successMessage.value = 'Entity definition updated successfully'
        } catch (err) {
            showErrorSnackbar.value = true
            errorMessage.value =
                err instanceof Error ? err.message : t('entity_definitions.edit.error')
        } finally {
            updating.value = false
        }
    }

    const deleteEntityDefinition = async () => {
        if (!selectedDefinition.value) {
            return
        }

        deleting.value = true

        try {
            await typedHttpClient.deleteEntityDefinition(selectedDefinition.value.uuid!)

            // Remove from list
            entityDefinitions.value = entityDefinitions.value.filter(
                d => d.uuid !== selectedDefinition.value?.uuid
            )
            selectedDefinition.value = null
            selectedItems.value = []

            showDeleteDialog.value = false

            showSuccessSnackbar.value = true
            successMessage.value = t('entity_definitions.delete.success')
        } catch (err) {
            showErrorSnackbar.value = true
            errorMessage.value =
                err instanceof Error ? err.message : t('entity_definitions.delete.error')
        } finally {
            deleting.value = false
        }
    }

    const applySchema = async () => {
        if (!selectedDefinition.value) {
            return
        }

        applyingSchema.value = true

        try {
            await typedHttpClient.applyEntityDefinitionSchema(selectedDefinition.value.uuid!)

            showSuccessSnackbar.value = true
            successMessage.value = t('entity_definitions.details.schema_applied')
        } catch (err) {
            showErrorSnackbar.value = true
            errorMessage.value =
                err instanceof Error ? err.message : t('entity_definitions.details.schema_error')
        } finally {
            applyingSchema.value = false
        }
    }

    const addField = () => {
        // This would open a field creation dialog
        console.log('Add field')
    }

    const editField = (field: FieldDefinition) => {
        // This would open a field editing dialog
        console.log('Edit field', field)
    }

    const removeField = (field: FieldDefinition) => {
        // This would remove the field from the selected definition
        console.log('Remove field', field)
    }

    const addFieldToEdit = () => {
        // Add a new field to the edit form
        const newField: FieldDefinition = {
            name: '',
            display_name: '',
            field_type: 'String',
            description: '',
            required: false,
            indexed: false,
            filterable: false,
            default_value: undefined,
            constraints: undefined,
            ui_settings: undefined,
        }
        editForm.value.fields.push(newField)
    }

    const editFieldInEdit = (field: FieldDefinition) => {
        // This would open a field editing dialog for the edit form
        console.log('Edit field in edit', field)
    }

    const removeFieldFromEdit = (field: FieldDefinition) => {
        const index = editForm.value.fields.findIndex(f => f.name === field.name)
        if (index !== -1) {
            editForm.value.fields.splice(index, 1)
        }
    }

    const formatDate = (dateString?: string) => {
        if (!dateString) {
            return 'N/A'
        }
        return new Date(dateString).toLocaleDateString()
    }

    // Lifecycle
    onMounted(() => {
        isComponentMounted.value = true
        loadEntityDefinitions()
    })

    onUnmounted(() => {
        isComponentMounted.value = false
    })
</script>
