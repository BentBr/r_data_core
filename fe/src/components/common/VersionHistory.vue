<template>
    <div class="version-history">
        <div
            v-if="versions.length === 0"
            class="text-grey text-body-2"
        >
            {{ t('entities.details.no_versions') }}
        </div>
        <div v-else>
            <div class="mb-4">
                <div class="text-subtitle-2 mb-2">
                    {{ t('entities.details.select_versions') }}
                </div>
                <v-list
                    density="compact"
                    class="version-list"
                >
                    <v-list-item
                        v-for="version in versions"
                        :key="version.version_number"
                        :class="{
                            'version-selected': isVersionSelected(version.version_number),
                            'version-item': true,
                        }"
                        @click="toggleVersionSelection(version.version_number)"
                    >
                        <template #prepend>
                            <v-checkbox
                                :model-value="isVersionSelected(version.version_number)"
                                density="compact"
                                hide-details
                                color="primary"
                                :disabled="
                                    !isVersionSelected(version.version_number) &&
                                    selectedVersions.length >= 2
                                "
                                @click.stop="toggleVersionSelection(version.version_number)"
                            />
                        </template>
                        <v-list-item-title>
                            Version {{ version.version_number }}
                        </v-list-item-title>
                        <v-list-item-subtitle>
                            {{ new Date(version.created_at).toLocaleString() }}
                            <span v-if="version.created_by_name">
                                • {{ version.created_by_name }}
                            </span>
                        </v-list-item-subtitle>
                    </v-list-item>
                </v-list>
            </div>

            <v-divider class="my-4" />

            <div v-if="diffRows.length > 0">
                <div class="text-subtitle-2 mb-2">
                    {{ t('entities.details.differences') }}
                </div>
                <v-table
                    density="compact"
                    class="entity-diff-table"
                >
                    <thead>
                        <tr>
                            <th>{{ t('entities.details.diff_field') }}</th>
                            <th>
                                {{ t('entities.details.version_a') }} ({{ selectedVersions[0] }})
                            </th>
                            <th>
                                {{ t('entities.details.version_b') }} ({{ selectedVersions[1] }})
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="row in diffRows"
                            :key="row.field"
                            :class="{ changed: row.changed }"
                        >
                            <td class="field">{{ row.field }}</td>
                            <td class="val">{{ row.a }}</td>
                            <td class="val">{{ row.b }}</td>
                        </tr>
                    </tbody>
                </v-table>
            </div>
            <div
                v-else-if="selectedVersions.length === 2"
                class="text-grey text-body-2"
            >
                {{ t('entities.details.no_diff') }}
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
    import { ref, watch, computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'

    interface Version {
        version_number: number
        created_at: string
        created_by?: string | null
        created_by_name?: string | null
    }

    interface DiffRow {
        field: string
        a: string
        b: string
        changed: boolean
    }

    interface Props {
        versions: Version[]
        loading?: boolean
    }

    interface Emits {
        (e: 'compare', versionA: number, versionB: number): void
    }

    const props = withDefaults(defineProps<Props>(), {
        loading: false,
    })

    const emit = defineEmits<Emits>()
    const { t } = useTranslations()

    const selectedVersions = ref<number[]>([])
    const diffRows = ref<DiffRow[]>([])

    const isVersionSelected = (versionNumber: number): boolean => {
        return selectedVersions.value.includes(versionNumber)
    }

    const toggleVersionSelection = async (versionNumber: number) => {
        const index = selectedVersions.value.indexOf(versionNumber)

        if (index > -1) {
            // Deselect
            selectedVersions.value.splice(index, 1)
            diffRows.value = []
        } else if (selectedVersions.value.length < 2) {
            // Select (only if less than 2 already selected)
            selectedVersions.value.push(versionNumber)

            // If we now have 2 selected, trigger comparison
            if (selectedVersions.value.length === 2) {
                emit('compare', selectedVersions.value[0], selectedVersions.value[1])
            }
        }
    }

    // Method to update diff rows (called by parent component)
    const updateDiffRows = (rows: DiffRow[]) => {
        diffRows.value = rows
    }

    // Reset when versions change
    watch(
        () => props.versions,
        () => {
            selectedVersions.value = []
            diffRows.value = []
        }
    )

    // Expose methods for parent components
    defineExpose({
        updateDiffRows,
        selectedVersions: computed(() => selectedVersions.value),
    })
</script>

<style scoped>
    .version-history {
        margin-top: 1rem;
    }

    .version-list {
        max-height: 400px;
        overflow-y: auto;
        border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
        border-radius: 4px;
    }

    .version-item {
        cursor: pointer;
        transition: background-color 0.2s;
    }

    .version-item:hover {
        background-color: rgba(var(--v-theme-on-surface), 0.04);
    }

    .version-selected {
        background-color: rgba(var(--v-theme-primary), 0.08);
    }

    .entity-diff-table {
        border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
    }

    .entity-diff-table .changed {
        background-color: rgba(var(--v-theme-warning), 0.1);
    }

    .entity-diff-table .field {
        font-weight: 500;
    }

    .entity-diff-table .val {
        font-family: monospace;
        font-size: 0.875rem;
    }
</style>
