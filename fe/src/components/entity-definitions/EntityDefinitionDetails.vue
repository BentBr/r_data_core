<template>
    <v-card
        v-if="definition"
        variant="outlined"
    >
        <v-card-title class="d-flex align-center justify-space-between pa-4">
            <div class="d-flex align-center">
                <SmartIcon
                    :icon="definition.icon ?? 'file-text'"
                    :size="28"
                    class="mr-3"
                />
                <span class="text-h5">{{ definition.display_name }}</span>
            </div>
            <div>
                <v-btn
                    color="primary"
                    variant="outlined"
                    class="mr-2"
                    @click="$emit('edit')"
                >
                    <template #prepend>
                        <SmartIcon
                            icon="pencil"
                            :size="20"
                        />
                    </template>
                    Edit
                </v-btn>

                <v-btn
                    color="error"
                    variant="outlined"
                    @click="$emit('delete')"
                >
                    <template #prepend>
                        <SmartIcon
                            icon="trash-2"
                            :size="20"
                        />
                    </template>
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
                    <VersionHistory
                        ref="versionHistoryRef"
                        :versions="versions"
                        @compare="handleVersionCompare"
                    />
                </v-window-item>
            </v-window>
        </v-card-text>
    </v-card>

    <v-card
        v-else
        variant="outlined"
    >
        <v-card-text class="text-center pa-8">
            <SmartIcon
                icon="file-text"
                :size="64"
                color="mutedForeground"
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
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import VersionHistory from '@/components/common/VersionHistory.vue'
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
    const versions = ref<
        Array<{
            version_number: number
            created_at: string
            created_by?: string | null
            created_by_name?: string | null
        }>
    >([])
    const versionHistoryRef = ref<InstanceType<typeof VersionHistory>>()

    const loadVersions = async () => {
        if (!props.definition?.uuid) {
            return
        }
        try {
            const uuid = props.definition.uuid
            versions.value = await typedHttpClient.listEntityDefinitionVersions(uuid)
        } catch (e) {
            console.error('Failed to load versions:', e)
        }
    }

    const handleVersionCompare = async (versionA: number, versionB: number) => {
        if (!props.definition?.uuid) {
            return
        }
        const uuid = props.definition.uuid
        try {
            const [a, b] = await Promise.all([
                typedHttpClient.getEntityDefinitionVersion(uuid, versionA),
                typedHttpClient.getEntityDefinitionVersion(uuid, versionB),
            ])
            const aData = a.data as Record<string, unknown>
            const bData = b.data as Record<string, unknown>
            const diffRows = computeDiffRows(aData, bData)
            versionHistoryRef.value?.updateDiffRows(diffRows)
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

    // Reload versions when switching to history tab
    watch(
        () => activeTab.value,
        async newTab => {
            if (newTab === 'history' && props.definition?.uuid) {
                await loadVersions()
            }
        }
    )
</script>

<style scoped></style>
