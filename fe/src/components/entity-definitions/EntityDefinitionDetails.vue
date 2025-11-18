<template>
    <v-card
        v-if="definition"
        variant="outlined"
    >
        <v-card-title class="d-flex align-center justify-space-between pa-4">
            <div class="d-flex align-center">
                <v-icon
                    :icon="definition.icon ?? 'mdi-file-document'"
                    class="mr-3"
                />
                <span class="text-h5">{{ definition.display_name }}</span>
            </div>
            <div>
                <v-btn
                    color="primary"
                    variant="outlined"
                    prepend-icon="mdi-pencil"
                    class="mr-2"
                    @click="$emit('edit')"
                >
                    Edit
                </v-btn>

                <v-btn
                    color="error"
                    variant="outlined"
                    prepend-icon="mdi-delete"
                    @click="$emit('delete')"
                >
                    {{ t('entity_definitions.delete.button') }}
                </v-btn>
            </div>
        </v-card-title>

        <v-card-text>
            <v-tabs v-model="activeTab">
                <v-tab value="meta">{{ t('entity_definitions.details.meta_info') }}</v-tab>
                <v-tab value="fields">{{ t('entity_definitions.details.fields') }}</v-tab>
                <v-tab value="history">{{ t('entities.details.history') }}</v-tab>
            </v-tabs>

            <v-window v-model="activeTab">
                <!-- Meta Information Tab -->
                <v-window-item value="meta">
                    <EntityDefinitionMetaInfo :definition="definition" />
                </v-window-item>

                <!-- Fields Tab -->
                <v-window-item value="fields">
                    <EntityDefinitionFields
                        :definition="definition"
                        :has-unsaved-changes="hasUnsavedChanges"
                        :saving-changes="savingChanges"
                        @save-changes="$emit('save-changes')"
                        @add-field="$emit('add-field')"
                        @edit-field="$emit('edit-field', $event)"
                        @remove-field="$emit('remove-field', $event)"
                    />
                </v-window-item>

                <!-- History Tab -->
                <v-window-item value="history">
                    <div v-if="versions.length === 0" class="text-grey text-body-2">
                        {{ t('entities.details.no_versions') }}
                    </div>
                    <div v-else>
                        <div class="mb-4">
                            <div class="text-subtitle-2 mb-2">Select two versions to compare:</div>
                            <v-list density="compact" class="version-list">
                                <v-list-item
                                    v-for="version in versions"
                                    :key="version.version_number"
                                    :class="{
                                        'version-selected': isVersionSelected(version.version_number),
                                        'version-item': true
                                    }"
                                    @click="toggleVersionSelection(version.version_number)"
                                >
                                    <template v-slot:prepend>
                                        <v-checkbox
                                            :model-value="isVersionSelected(version.version_number)"
                                            density="compact"
                                            hide-details
                                            @click.stop="toggleVersionSelection(version.version_number)"
                                        />
                                    </template>
                                    <v-list-item-title>
                                        Version {{ version.version_number }}
                                    </v-list-item-title>
                                    <v-list-item-subtitle>
                                        {{ formatDate(version.created_at) }}
                                        <span v-if="version.created_by_name">
                                            • {{ version.created_by_name }}
                                        </span>
                                    </v-list-item-subtitle>
                                </v-list-item>
                            </v-list>
                        </div>
                        <v-divider class="my-4" />
                        <div v-if="diffRows.length === 0 && selectedA !== null && selectedB !== null" class="text-grey text-body-2">
                            {{ t('entities.details.no_diff') }}
                        </div>
                        <v-table
                            v-else-if="diffRows.length > 0"
                            density="compact"
                            class="entity-diff-table"
                        >
                            <thead>
                                <tr>
                                    <th class="text-left">Field</th>
                                    <th class="text-left">Version {{ selectedA }}</th>
                                    <th class="text-left">Version {{ selectedB }}</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr
                                    v-for="row in diffRows"
                                    :key="row.field"
                                    :class="row.changed ? 'changed' : ''"
                                >
                                    <td class="field">{{ row.field }}</td>
                                    <td class="val">{{ row.a }}</td>
                                    <td class="val">{{ row.b }}</td>
                                </tr>
                            </tbody>
                        </v-table>
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
</template>

<script setup lang="ts">
    import { ref, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import EntityDefinitionMetaInfo from './EntityDefinitionMetaInfo.vue'
    import EntityDefinitionFields from './EntityDefinitionFields.vue'
    import type { EntityDefinition } from '@/types/schemas'
    import { typedHttpClient } from '@/api/typed-client'
    import { computeDiffRows } from '@/utils/versionDiff'

    interface Props {
        definition: EntityDefinition | null
        hasUnsavedChanges: boolean
        savingChanges: boolean
    }

    interface Emits {
        (e: 'edit'): void
        (e: 'delete'): void
        (e: 'save-changes'): void
        (e: 'add-field'): void
        (e: 'edit-field', field: import('@/types/schemas').FieldDefinition): void
        (e: 'remove-field', field: import('@/types/schemas').FieldDefinition): void
    }

    const props = defineProps<Props>()
    defineEmits<Emits>()
    const { t } = useTranslations()

    const activeTab = ref('meta')

    // Versions/diff
    const versions = ref<Array<{ version_number: number; created_at: string; created_by?: string | null; created_by_name?: string | null }>>([])
    const selectedA = ref<number | null>(null)
    const selectedB = ref<number | null>(null)
    const diffRows = ref<Array<{ field: string; a: string; b: string; changed: boolean }>>([])

    const loadVersions = async () => {
        if (!props.definition || !props.definition.uuid) return
        try {
            const uuid = props.definition.uuid
            versions.value = await typedHttpClient.listEntityDefinitionVersions(uuid)
            selectedA.value = null
            selectedB.value = null
            diffRows.value = []
        } catch (e) {
            // ignore
        }
    }

    const isVersionSelected = (versionNumber: number): boolean => {
        return selectedA.value === versionNumber || selectedB.value === versionNumber
    }

    const toggleVersionSelection = async (versionNumber: number) => {
        if (selectedA.value === versionNumber) {
            // Deselect A
            selectedA.value = selectedB.value
            selectedB.value = null
        } else if (selectedB.value === versionNumber) {
            // Deselect B
            selectedB.value = null
        } else if (selectedA.value === null) {
            // Select as A
            selectedA.value = versionNumber
        } else if (selectedB.value === null) {
            // Select as B
            selectedB.value = versionNumber
            // Auto-load diff when both are selected
            await loadDiff()
        } else {
            // Both are selected, replace A with this version
            selectedA.value = versionNumber
            await loadDiff()
        }
    }

    const loadDiff = async () => {
        diffRows.value = []
        if (!props.definition || !props.definition.uuid || selectedA.value === null || selectedB.value === null) return
        const uuid = props.definition.uuid
        try {
            const [a, b] = await Promise.all([
                typedHttpClient.getEntityDefinitionVersion(uuid, selectedA.value),
                typedHttpClient.getEntityDefinitionVersion(uuid, selectedB.value),
            ])
            diffRows.value = computeDiffRows(
                (a.data as Record<string, unknown>) || {},
                (b.data as Record<string, unknown>) || {}
            )
        } catch (e) {
            console.error('Failed to load diff:', e)
        }
    }

    watch(
        () => props.definition?.uuid,
        async () => {
            await loadVersions()
        },
        { immediate: true }
    )

    const formatDate = (dateString: string) => {
        return new Date(dateString).toLocaleString()
    }
</script>

<style scoped>
.version-list {
    max-height: 400px;
    overflow-y: auto;
}

.version-item {
    cursor: pointer;
    transition: background-color 0.2s;
}

.version-item:hover {
    background-color: rgba(0, 0, 0, 0.04);
}

.version-selected {
    background-color: rgba(25, 118, 210, 0.08);
}

.entity-diff-table .changed {
    background-color: rgba(255, 193, 7, 0.1);
}

.entity-diff-table .field {
    font-weight: 500;
}

.entity-diff-table .val {
    font-family: monospace;
    font-size: 0.875rem;
}
</style>
