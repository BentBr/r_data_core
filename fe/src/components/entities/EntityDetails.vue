<template>
    <div class="entity-details">
        <div
            v-if="!entity"
            class="d-flex justify-center align-center pa-8"
        >
            <div class="text-center">
                <v-icon
                    icon="mdi-database-off"
                    size="large"
                    color="grey"
                    class="mb-3"
                />
                <p class="text-grey">{{ t('entities.details.select_entity') }}</p>
            </div>
        </div>

        <div v-else>
            <!-- Header -->
            <div class="d-flex justify-space-between align-center mb-4">
                <div>
                    <h3 class="text-h5">{{ t('entities.details.title') }}</h3>
                    <p class="text-subtitle-1 text-grey">
                        {{ entityDefinition?.display_name ?? entity.entity_type }}
                    </p>
                </div>
                <div class="d-flex gap-2">
                    <v-btn
                        color="primary"
                        variant="outlined"
                        prepend-icon="mdi-pencil"
                        @click="$emit('edit')"
                    >
                        {{ t('common.edit') }}
                    </v-btn>
                    <v-btn
                        color="error"
                        variant="outlined"
                        prepend-icon="mdi-delete"
                        @click="$emit('delete')"
                    >
                        {{ t('common.delete') }}
                    </v-btn>
                </div>
            </div>

            <v-divider class="mb-4" />

            <!-- Entity Information -->
            <v-row>
                <v-col cols="6">
                    <v-card variant="outlined">
                        <v-card-title class="text-subtitle-1 pa-3">
                            {{ t('entities.details.basic_info') }}
                        </v-card-title>
                        <v-card-text class="pa-3">
                            <v-list density="compact">
                                <v-list-item>
                                    <template #prepend>
                                        <v-icon
                                            icon="mdi-identifier"
                                            size="small"
                                        />
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.uuid')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>{{
                                        entity.field_data?.uuid ?? ''
                                    }}</v-list-item-subtitle>
                                </v-list-item>
                                <v-list-item>
                                    <template #prepend>
                                        <v-icon
                                            icon="mdi-database"
                                            size="small"
                                        />
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.entity_type')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>{{
                                        entity.entity_type
                                    }}</v-list-item-subtitle>
                                </v-list-item>
                                <v-list-item>
                                    <template #prepend>
                                        <v-icon
                                            icon="mdi-calendar-plus"
                                            size="small"
                                        />
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.created_at')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>{{
                                        formatDate(entity.field_data?.created_at ?? '')
                                    }}</v-list-item-subtitle>
                                </v-list-item>
                                <v-list-item>
                                    <template #prepend>
                                        <v-icon
                                            icon="mdi-calendar-edit"
                                            size="small"
                                        />
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.updated_at')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>{{
                                        formatDate(entity.field_data?.updated_at ?? '')
                                    }}</v-list-item-subtitle>
                                </v-list-item>
                                <v-list-item v-if="entity.field_data?.path">
                                    <template #prepend>
                                        <v-icon
                                            icon="mdi-route"
                                            size="small"
                                        />
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.path')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>{{
                                        entity.field_data?.path
                                    }}</v-list-item-subtitle>
                                </v-list-item>
                            </v-list>
                        </v-card-text>
                    </v-card>
                </v-col>

                <v-col cols="6">
                    <v-card variant="outlined">
                        <v-card-title class="text-subtitle-1 pa-3">
                            {{ t('entities.details.relationships') }}
                        </v-card-title>
                        <v-card-text class="pa-3">
                            <v-list density="compact">
                                <v-list-item>
                                    <template #prepend>
                                        <v-icon
                                            icon="mdi-arrow-up"
                                            size="small"
                                        />
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.parent')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>
                                        {{
                                            entity.field_data?.parent_uuid ||
                                            t('entities.details.no_parent')
                                        }}
                                    </v-list-item-subtitle>
                                </v-list-item>
                                <v-list-item>
                                    <template #prepend>
                                        <v-icon
                                            icon="mdi-arrow-down"
                                            size="small"
                                        />
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.children')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>
                                        0
                                        {{ t('entities.details.child_count') }}
                                    </v-list-item-subtitle>
                                </v-list-item>
                            </v-list>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>

            <!-- Entity Data -->
            <v-card
                variant="outlined"
                class="mt-4"
            >
                <v-card-title class="text-subtitle-1 pa-3">
                    {{ t('entities.details.data') }}
                </v-card-title>
                <v-card-text class="pa-3">
                    <v-expansion-panels
                        variant="accordion"
                        :model-value="entityDefinition ? [0, 1, 2] : [0, 2]"
                    >
                        <v-expansion-panel v-if="entityDefinition">
                            <v-expansion-panel-title>
                                {{ t('entities.details.formatted_data') }}
                            </v-expansion-panel-title>
                            <v-expansion-panel-text>
                                <v-list density="compact">
                                    <v-list-item
                                        v-for="field in entityDefinition.fields"
                                        :key="field.name"
                                    >
                                        <template #prepend>
                                            <v-icon
                                                :icon="getFieldIcon(field.field_type)"
                                                size="small"
                                            />
                                        </template>
                                        <v-list-item-title>{{
                                            field.display_name
                                        }}</v-list-item-title>
                                        <v-list-item-subtitle>
                                            {{
                                                formatFieldValue(
                                                    resolveFieldValue(
                                                        entity.field_data ?? {},
                                                        field.name
                                                    ),
                                                    field.field_type
                                                )
                                            }}
                                        </v-list-item-subtitle>
                                    </v-list-item>
                                </v-list>
                            </v-expansion-panel-text>
                        </v-expansion-panel>

                        <v-expansion-panel>
                            <v-expansion-panel-title>
                                {{ t('entities.details.raw_data') }}
                            </v-expansion-panel-title>
                            <v-expansion-panel-text>
                                <pre class="text-body-2 bg-grey-lighten-4 pa-3 rounded">{{
                                    JSON.stringify(entity.field_data, null, 2)
                                }}</pre>
                            </v-expansion-panel-text>
                        </v-expansion-panel>
                        <v-expansion-panel v-if="entity">
                            <v-expansion-panel-title>
                                {{ t('entities.details.history') }}
                            </v-expansion-panel-title>
                            <v-expansion-panel-text>
                                <div class="mb-3 d-flex align-center" style="gap: 8px">
                                    <v-select
                                        v-model="selectedA"
                                        :items="versions"
                                        item-title="version_number"
                                        item-value="version_number"
                                        label="Version A"
                                        density="compact"
                                        style="max-width: 180px"
                                    />
                                    <v-select
                                        v-model="selectedB"
                                        :items="versions"
                                        item-title="version_number"
                                        item-value="version_number"
                                        label="Version B"
                                        density="compact"
                                        style="max-width: 180px"
                                    />
                                    <v-btn
                                        variant="outlined"
                                        color="primary"
                                        density="comfortable"
                                        @click="loadDiff"
                                    >
                                        Compare
                                    </v-btn>
                                </div>
                                <div v-if="diffRows.length === 0" class="text-grey text-body-2">
                                    {{ t('entities.details.no_diff') }}
                                </div>
                                <v-table
                                    v-else
                                    density="compact"
                                    class="entity-diff-table"
                                >
                                    <thead>
                                        <tr>
                                            <th class="text-left">Field</th>
                                            <th class="text-left">A</th>
                                            <th class="text-left">B</th>
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
                            </v-expansion-panel-text>
                        </v-expansion-panel>
                    </v-expansion-panels>
                </v-card-text>
            </v-card>
        </div>
    </div>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'
    import type { DynamicEntity, EntityDefinition } from '@/types/schemas'
    import { typedHttpClient } from '@/api/typed-client'
    import { ref, watch } from 'vue'

    interface Props {
        entity: DynamicEntity | null
        entityDefinition: EntityDefinition | null
    }

    interface Emits {
        (e: 'edit'): void
        (e: 'delete'): void
    }

    const props = defineProps<Props>()
    defineEmits<Emits>()

    const { t } = useTranslations()

    const toToken = (s: string): string =>
        (s || '')
            .toLowerCase()
            .split(/[^a-z0-9]+/g)
            .filter(Boolean)
            .join('')

    const resolveFieldValue = (data: Record<string, unknown>, fieldName: string): unknown => {
        if (!data) {
            return undefined
        }
        // 1) exact
        if (fieldName in data) {
            return (data as any)[fieldName]
        }
        // 2) case-insensitive
        const lower = fieldName.toLowerCase()
        for (const k of Object.keys(data)) {
            if (k.toLowerCase() === lower) {
                return (data as any)[k]
            }
        }
        // 3) token-based (firstname vs first_name vs FirstName)
        const wanted = toToken(fieldName)
        for (const k of Object.keys(data)) {
            if (toToken(k) === wanted) {
                return (data as any)[k]
            }
        }
        return undefined
    }

    // Versions/diff
    const versions = ref<Array<{ version_number: number; created_at: string; created_by?: string | null }>>([])
    const selectedA = ref<number | null>(null)
    const selectedB = ref<number | null>(null)
    const diffRows = ref<Array<{ field: string; a: string; b: string; changed: boolean }>>([])

    const loadVersions = async () => {
        if (!props.entity) return
        try {
            const uuid = String(props.entity.field_data?.uuid || '')
            const entityType = props.entity.entity_type
            versions.value = await typedHttpClient.listEntityVersions(entityType, uuid)
            if (versions.value.length >= 2) {
                selectedA.value = versions.value[1]?.version_number ?? versions.value[0]?.version_number ?? null
                selectedB.value = versions.value[0]?.version_number ?? null
            } else if (versions.value.length === 1) {
                selectedA.value = versions.value[0].version_number
                selectedB.value = versions.value[0].version_number
            }
            diffRows.value = []
        } catch (e) {
            // ignore
        }
    }

    import { computeDiffRows } from '@/utils/versionDiff'

    const loadDiff = async () => {
        diffRows.value = []
        if (!props.entity || selectedA.value === null || selectedB.value === null) return
        const uuid = String(props.entity.field_data?.uuid || '')
        const entityType = props.entity.entity_type
        const [a, b] = await Promise.all([
            typedHttpClient.getEntityVersion(entityType, uuid, selectedA.value),
            typedHttpClient.getEntityVersion(entityType, uuid, selectedB.value),
        ])
        diffRows.value = computeDiffRows(
            (a.data as Record<string, unknown>) || {},
            (b.data as Record<string, unknown>) || {}
        )
    }

    watch(
        () => props.entity?.field_data?.uuid,
        async () => {
            await loadVersions()
        },
        { immediate: true }
    )

    const formatDate = (dateString: string) => {
        return new Date(dateString).toLocaleString()
    }

    const getFieldIcon = (fieldType: string) => {
        const iconMap: Record<string, string> = {
            String: 'mdi-text',
            Text: 'mdi-text-box',
            Wysiwyg: 'mdi-text-box-edit',
            Integer: 'mdi-numeric',
            Float: 'mdi-decimal',
            Boolean: 'mdi-checkbox-marked',
            Date: 'mdi-calendar',
            DateTime: 'mdi-calendar-clock',
            Time: 'mdi-clock',
            Email: 'mdi-email',
            Url: 'mdi-link',
            File: 'mdi-file',
            Image: 'mdi-image',
            Json: 'mdi-code-json',
        }
        return iconMap[fieldType] || 'mdi-text'
    }

    const formatFieldValue = (value: unknown, fieldType: string): string => {
        if (value === null || value === undefined) {
            return t('common.empty')
        }

        switch (fieldType) {
            case 'Boolean':
                return value ? t('common.yes') : t('common.no')
            case 'Date':
            case 'DateTime':
                return new Date(value as string).toLocaleDateString()
            case 'Time':
                return new Date(`2000-01-01T${value}`).toLocaleTimeString()
            case 'Json':
                return typeof value === 'object' ? JSON.stringify(value) : String(value)
            default:
                return String(value)
        }
    }
</script>

<style scoped>
    .entity-details {
        height: 100%;
        overflow-y: auto;
    }

    pre {
        white-space: pre-wrap;
        word-wrap: break-word;
        max-height: 300px;
        overflow-y: auto;
    }
</style>
