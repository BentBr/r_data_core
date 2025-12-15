<template>
    <div class="entity-details">
        <div
            v-if="!entity"
            class="d-flex justify-center align-center pa-8"
        >
            <div class="text-center">
                <SmartIcon
                    icon="database"
                    :size="48"
                    color="mutedForeground"
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
                        @click="$emit('edit')"
                    >
                        <template #prepend>
                            <SmartIcon
                                icon="pencil"
                                :size="20"
                            />
                        </template>
                        {{ t('common.edit') }}
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
                                        <div class="mr-3">
                                            <SmartIcon
                                                icon="hash"
                                                :size="20"
                                            />
                                        </div>
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
                                        <div class="mr-3">
                                            <SmartIcon
                                                icon="database"
                                                :size="20"
                                            />
                                        </div>
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
                                        <div class="mr-3">
                                            <SmartIcon
                                                icon="calendar-plus"
                                                :size="20"
                                            />
                                        </div>
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.created_at')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>{{
                                        formatDate((entity.field_data?.created_at as string) ?? '')
                                    }}</v-list-item-subtitle>
                                </v-list-item>
                                <v-list-item>
                                    <template #prepend>
                                        <div class="mr-3">
                                            <SmartIcon
                                                icon="calendar"
                                                :size="20"
                                            />
                                        </div>
                                    </template>
                                    <v-list-item-title>{{
                                        t('entities.details.updated_at')
                                    }}</v-list-item-title>
                                    <v-list-item-subtitle>{{
                                        formatDate((entity.field_data?.updated_at as string) ?? '')
                                    }}</v-list-item-subtitle>
                                </v-list-item>
                                <v-list-item v-if="entity.field_data?.path">
                                    <template #prepend>
                                        <div class="mr-3">
                                            <SmartIcon
                                                icon="route"
                                                :size="20"
                                            />
                                        </div>
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
                                        <div class="mr-3">
                                            <SmartIcon
                                                icon="arrow-up"
                                                :size="20"
                                            />
                                        </div>
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
                                        <div class="mr-3">
                                            <SmartIcon
                                                icon="arrow-down"
                                                :size="20"
                                            />
                                        </div>
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
                        :model-value="entityDefinition ? [0] : []"
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
                                            <div class="mr-3">
                                                <SmartIcon
                                                    :icon="getFieldIcon(field.field_type)"
                                                    :size="20"
                                                />
                                            </div>
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
                                <VersionHistory
                                    ref="versionHistoryRef"
                                    :versions="versions"
                                    @compare="handleVersionCompare"
                                />
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
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import VersionHistory from '@/components/common/VersionHistory.vue'
    import type { DynamicEntity, EntityDefinition } from '@/types/schemas'
    import { typedHttpClient } from '@/api/typed-client'
    import { ref, watch } from 'vue'
    import { computeDiffRows } from '@/utils/versionDiff'

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
            return (data as Record<string, unknown>)[fieldName]
        }
        // 2) case-insensitive
        const lower = fieldName.toLowerCase()
        for (const k of Object.keys(data)) {
            if (k.toLowerCase() === lower) {
                return (data as Record<string, unknown>)[k]
            }
        }
        // 3) token-based (firstname vs first_name vs FirstName)
        const wanted = toToken(fieldName)
        for (const k of Object.keys(data)) {
            if (toToken(k) === wanted) {
                return (data as Record<string, unknown>)[k]
            }
        }
        return undefined
    }

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
        if (!props.entity) {
            return
        }
        try {
            const uuid = String(props.entity.field_data?.uuid ?? '')
            const entityType = props.entity.entity_type
            versions.value = await typedHttpClient.listEntityVersions(entityType, uuid)
        } catch (e) {
            console.error('Failed to load versions:', e)
        }
    }

    const handleVersionCompare = async (versionA: number, versionB: number) => {
        if (!props.entity) {
            return
        }
        const uuid = String(props.entity.field_data?.uuid ?? '')
        const entityType = props.entity.entity_type
        try {
            const [a, b] = await Promise.all([
                typedHttpClient.getEntityVersion(entityType, uuid, versionA),
                typedHttpClient.getEntityVersion(entityType, uuid, versionB),
            ])
            const diffRows = computeDiffRows(
                (a.data as Record<string, unknown>) ?? {},
                (b.data as Record<string, unknown>) ?? {}
            )
            versionHistoryRef.value?.updateDiffRows(diffRows)
        } catch (e) {
            console.error('Failed to load diff:', e)
        }
    }

    watch(
        () => props.entity?.field_data?.uuid,
        async () => {
            await loadVersions()
        },
        { immediate: true }
    )

    const getFieldIcon = (fieldType: string) => {
        const iconMap: Record<string, string> = {
            String: 'type',
            Text: 'file-text',
            Wysiwyg: 'file-edit',
            Integer: 'hash',
            Float: 'hash',
            Boolean: 'check-square',
            Date: 'calendar',
            DateTime: 'calendar-clock',
            Time: 'clock',
            Email: 'mail',
            Url: 'link',
            File: 'file',
            Image: 'image',
            Json: 'code',
        }
        return iconMap[fieldType] || 'type'
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

    const formatDate = (value: string | undefined | null): string => {
        if (!value) {
            return ''
        }
        const date = new Date(value)
        return Number.isNaN(date.getTime()) ? '' : date.toLocaleString()
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
