<template>
    <div>
        <div class="text-subtitle-2 font-weight-bold mb-2">{{ t('workflows.dsl.from') }}</div>
        <v-select
            :model-value="modelValue.type"
            :items="fromTypes"
            :label="t('workflows.dsl.from_type')"
            density="comfortable"
            class="mb-2"
            @update:model-value="onTypeChange"
        />
        <template v-if="modelValue.type === 'format'">
            <div class="d-flex ga-2 mb-2 flex-wrap">
                <v-select
                    :model-value="sourceType"
                    :items="sourceTypes"
                    :label="t('workflows.dsl.source_type')"
                    density="comfortable"
                    @update:model-value="updateSourceType($event)"
                />
                <v-select
                    :model-value="formatType"
                    :items="formatTypes"
                    :label="t('workflows.dsl.format_type')"
                    density="comfortable"
                    @update:model-value="updateFormatType($event)"
                />
            </div>
            <template v-if="sourceType === 'uri'">
                <v-text-field
                    :model-value="sourceUri"
                    :label="t('workflows.dsl.uri')"
                    density="comfortable"
                    class="mb-2"
                    @update:model-value="updateSourceConfig('uri', $event)"
                />
            </template>
            <!-- from.api source type = Accept POST to this workflow (no endpoint field needed) -->
            <template v-if="sourceType === 'api'">
                <div
                    class="text-caption mb-2 pa-2"
                    style="background-color: rgba(var(--v-theme-primary), 0.1); border-radius: 4px"
                >
                    <strong>{{ t('workflows.dsl.endpoint_info') }}:</strong> POST
                    {{ getFullEndpointUri() }}
                </div>
            </template>
            <div
                v-if="formatType === 'csv'"
                class="mb-2"
            >
                <CsvOptionsEditor
                    :model-value="formatOptions"
                    @update:model-value="updateFormatOptions($event)"
                />
            </div>
            <div
                v-if="formatType === 'csv' && sourceType !== 'trigger'"
                class="mb-2"
            >
                <div class="d-flex align-center ga-2 flex-wrap">
                    <input
                        type="file"
                        accept=".csv,text/csv"
                        @change="onTestUpload"
                    />
                    <v-btn
                        v-if="sourceType === 'uri' && sourceUri"
                        size="x-small"
                        variant="tonal"
                        @click="autoMapFromUri"
                        >{{ t('workflows.dsl.auto_map_from_uri') }}
                    </v-btn>
                </div>
            </div>
            <div class="mb-2">
                <v-expansion-panels variant="accordion">
                    <v-expansion-panel>
                        <v-expansion-panel-title
                            >{{ t('workflows.dsl.auth_type') }}
                        </v-expansion-panel-title>
                        <v-expansion-panel-text>
                            <AuthConfigEditor
                                :model-value="sourceAuth"
                                @update:model-value="updateSourceAuth($event)"
                            />
                        </v-expansion-panel-text>
                    </v-expansion-panel>
                </v-expansion-panels>
            </div>
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_source_normalized') }}
            </div>
            <MappingEditor
                ref="mappingEditorRef"
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.source')"
                :right-label="t('workflows.dsl.normalized')"
                @update:model-value="updateField('mapping', $event)"
            />
            <v-btn
                size="x-small"
                variant="tonal"
                @click="addMapping"
                >{{ t('workflows.dsl.add_mapping') }}
            </v-btn>
        </template>
        <template v-else-if="modelValue.type === 'trigger'">
            <div
                class="text-caption mb-2 pa-2"
                style="background-color: rgba(var(--v-theme-primary), 0.1); border-radius: 4px"
            >
                <strong>{{ t('workflows.dsl.endpoint_info') }}:</strong> GET
                {{ getTriggerEndpointUri() }}
            </div>
        </template>
        <template v-else-if="modelValue.type === 'entity'">
            <v-select
                :model-value="entityDefinition"
                :items="entityDefItems"
                item-title="title"
                item-value="value"
                :label="t('workflows.dsl.entity_definition')"
                density="comfortable"
                @update:model-value="onEntityDefChange"
            />
            <div class="d-flex ga-2 flex-wrap align-center mb-2">
                <v-switch
                    :model-value="filterEnabled"
                    density="comfortable"
                    :label="t('workflows.dsl.apply_filter')"
                    color="primary"
                    @update:model-value="toggleFilter"
                />
            </div>
            <div
                v-if="filterEnabled"
                class="d-flex ga-2 flex-wrap"
            >
                <v-select
                    :model-value="filterField"
                    :items="filterFieldItems"
                    :label="t('workflows.dsl.filter_field')"
                    density="comfortable"
                    @update:model-value="updateFilterField('field', $event)"
                />
                <v-select
                    :model-value="filterOperator"
                    :items="[
                        { title: '=', value: '=' },
                        { title: '>', value: '>' },
                        { title: '<', value: '<' },
                        { title: '<=', value: '<=' },
                        { title: '>=', value: '>=' },
                        { title: 'IN', value: 'IN' },
                        { title: 'NOT IN', value: 'NOT IN' },
                    ]"
                    item-title="title"
                    item-value="value"
                    :label="t('workflows.dsl.filter_operator')"
                    density="comfortable"
                    @update:model-value="updateFilterField('operator', $event)"
                />
                <v-text-field
                    :model-value="filterValue"
                    :label="t('workflows.dsl.filter_value')"
                    density="comfortable"
                    @update:model-value="updateFilterField('value', $event)"
                />
            </div>
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_source_normalized') }}
            </div>
            <MappingEditor
                ref="mappingEditorRef"
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.source')"
                :right-label="t('workflows.dsl.normalized')"
                @update:model-value="updateField('mapping', $event)"
            />
            <v-btn
                size="x-small"
                variant="tonal"
                @click="addMapping"
                >{{ t('workflows.dsl.add_mapping') }}
            </v-btn>
        </template>
        <template v-else-if="modelValue.type === 'previous_step'">
            <!-- Info banner -->
            <v-alert
                v-if="stepIndex === 0"
                type="error"
                density="compact"
                class="mb-2"
            >
                {{ t('workflows.dsl.previous_step_error_first_step') }}
            </v-alert>
            <div
                v-else
                class="text-caption mb-2 pa-2"
                style="background-color: rgba(var(--v-theme-info), 0.1); border-radius: 4px"
            >
                <v-icon
                    size="small"
                    class="mr-1"
                    >mdi-arrow-up-circle
                </v-icon>
                {{ t('workflows.dsl.previous_step_info') }}
            </div>
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_previous_normalized') }}
            </div>
            <MappingEditor
                ref="mappingEditorRef"
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.previous_field')"
                :right-label="t('workflows.dsl.normalized')"
                :left-items="previousStepFields"
                :use-select-for-left="previousStepFields.length > 0"
                @update:model-value="updateField('mapping', $event)"
            />
            <v-btn
                size="x-small"
                variant="tonal"
                @click="addMapping"
                >{{ t('workflows.dsl.add_mapping') }}
            </v-btn>
        </template>
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, watch, onMounted } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useEntityDefinitions } from '@/composables/useEntityDefinitions'
    import { typedHttpClient } from '@/api/typed-client'
    import { buildApiUrl } from '@/env-check'
    import type { FromDef, AuthConfig } from './dsl-utils'
    import { defaultCsvOptions } from './dsl-utils'
    import CsvOptionsEditor from './CsvOptionsEditor.vue'
    import MappingEditor from './MappingEditor.vue'
    import AuthConfigEditor from './AuthConfigEditor.vue'

    const props = defineProps<{
        modelValue: FromDef
        workflowUuid?: string | null
        stepIndex?: number
        previousStepFields?: string[]
    }>()

    const emit = defineEmits<{ (e: 'update:modelValue', value: FromDef): void }>()

    const { t } = useTranslations()
    const { entityDefinitions, loadEntityDefinitions } = useEntityDefinitions()

    const mappingEditorRef = ref<{ addEmptyPair: () => void } | null>(null)
    const entityDefItems = ref<{ title: string; value: string }[]>([])
    const filterFieldItems = ref<string[]>([])

    // Default props
    const stepIndex = computed(() => props.stepIndex ?? 0)
    const previousStepFields = computed(() => props.previousStepFields ?? [])

    // Computed properties to avoid 'as any' in templates
    const sourceType = computed(() => {
        if (props.modelValue.type === 'format') {
            return props.modelValue.source.source_type
        }
        return 'uri'
    })

    const formatType = computed(() => {
        if (props.modelValue.type === 'format') {
            return props.modelValue.format.format_type
        }
        return 'csv'
    })

    const sourceUri = computed(() => {
        if (props.modelValue.type === 'format') {
            const config = props.modelValue.source.config
            if (typeof config === 'object' && 'uri' in config) {
                const uri = config.uri
                return uri != null ? String(uri) : ''
            }
        }
        return ''
    })

    const formatOptions = computed(() => {
        if (props.modelValue.type === 'format') {
            return props.modelValue.format.options ?? {}
        }
        return {}
    })

    const sourceAuth = computed(() => {
        if (props.modelValue.type === 'format') {
            return props.modelValue.source.auth ?? { type: 'none' as const }
        }
        return { type: 'none' as const }
    })

    const entityDefinition = computed(() => {
        if (props.modelValue.type === 'entity') {
            return props.modelValue.entity_definition
        }
        return ''
    })

    const filterField = computed(() => {
        if (props.modelValue.type === 'entity') {
            return props.modelValue.filter?.field ?? ''
        }
        return ''
    })

    const filterValue = computed(() => {
        if (props.modelValue.type === 'entity') {
            return props.modelValue.filter?.value ?? ''
        }
        return ''
    })

    const filterOperator = computed(() => {
        if (props.modelValue.type === 'entity') {
            return props.modelValue.filter?.operator ?? '='
        }
        return '='
    })

    const filterEnabled = computed(() => {
        return props.modelValue.type === 'entity' ? !!props.modelValue.filter : false
    })

    // Load entity definitions on mount
    watch(
        () => entityDefinitions.value,
        defs => {
            entityDefItems.value = defs.map(d => ({
                title: d.display_name || d.entity_type,
                value: d.entity_type,
            }))
        },
        { immediate: true }
    )

    // Load entity definitions when component is created
    onMounted(() => {
        void loadEntityDefinitions()
    })

    // Load entity fields when entity definition changes
    async function onEntityDefChange(entityType: string) {
        updateField('entity_definition', entityType)
        await loadEntityFields(entityType)
    }

    // Load entity fields helper
    async function loadEntityFields(entityType: string | null | undefined) {
        if (!entityType) {
            filterFieldItems.value = []
            return
        }
        try {
            const fields = await typedHttpClient.getEntityFields(entityType)
            // Include all fields including system fields
            filterFieldItems.value = fields.map(f => f.name)
        } catch {
            filterFieldItems.value = []
        }
    }

    // Load entity fields when entity definition is set initially
    watch(
        () => (props.modelValue.type === 'entity' ? props.modelValue.entity_definition : undefined),
        entityType => {
            if (props.modelValue.type === 'entity' && entityType) {
                void loadEntityFields(entityType)
            }
        },
        { immediate: true }
    )

    function getFullEndpointUri(): string {
        const uuid = props.workflowUuid ?? '{workflow-uuid}'
        return buildApiUrl(`/api/v1/workflows/${uuid}`)
    }

    function getTriggerEndpointUri(): string {
        const uuid = props.workflowUuid ?? '{workflow-uuid}'
        return buildApiUrl(`/api/v1/workflows/${uuid}/trigger`)
    }

    const fromTypes = [
        { title: 'Format (CSV/JSON)', value: 'format' },
        { title: 'Entity', value: 'entity' },
        { title: 'Previous Step', value: 'previous_step' },
        { title: 'Trigger', value: 'trigger' },
    ]

    function updateField(field: string, value: unknown) {
        const updated = { ...props.modelValue } as Record<string, unknown>
        updated[field] = value
        // Ensure entity mapping exists; filter is optional
        if (updated.type === 'entity') {
            updated.mapping ??= {}
        }
        emit('update:modelValue', updated as FromDef)
    }

    function updateFilterField(field: string, value: unknown) {
        const updated = { ...props.modelValue } as Record<string, unknown>
        updated.filter ??= { field: '', operator: '=', value: '' }
        ;(updated.filter as Record<string, unknown>)[field] = value
        emit('update:modelValue', updated as FromDef)
    }

    function toggleFilter(enabled: boolean | null) {
        const updated = { ...props.modelValue } as Record<string, unknown>
        if (enabled) {
            updated.filter ??= { field: '', operator: '=', value: '' }
        } else {
            delete updated.filter
        }
        emit('update:modelValue', updated as FromDef)
    }

    const sourceTypes = [
        { title: 'URI', value: 'uri' },
        { title: 'API', value: 'api' },
        { title: 'File', value: 'file' },
    ]

    const formatTypes = [
        { title: 'CSV', value: 'csv' },
        { title: 'JSON', value: 'json' },
    ]

    function updateSourceType(newType: string) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const updated: FromDef = {
            ...props.modelValue,
            source: {
                ...props.modelValue.source,
                source_type: newType,
                config:
                    newType === 'uri'
                        ? { uri: '' }
                        : newType === 'api' || newType === 'trigger'
                          ? {}
                          : {},
            },
        }
        emit('update:modelValue', updated)
    }

    function updateFormatType(newType: string) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const updated: FromDef = {
            ...props.modelValue,
            format: {
                ...props.modelValue.format,
                format_type: newType,
                options:
                    newType === 'csv' && !props.modelValue.format.options
                        ? defaultCsvOptions()
                        : props.modelValue.format.options,
            },
        }
        emit('update:modelValue', updated)
    }

    function updateSourceConfig(key: string, value: unknown) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const updated: FromDef = {
            ...props.modelValue,
            source: {
                ...props.modelValue.source,
                config: {
                    ...props.modelValue.source.config,
                    [key]: value,
                },
            },
        }
        emit('update:modelValue', updated)
    }

    function updateFormatOptions(options: Record<string, unknown>) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const updated: FromDef = {
            ...props.modelValue,
            format: {
                ...props.modelValue.format,
                options,
            },
        }
        emit('update:modelValue', updated)
    }

    function updateSourceAuth(auth: AuthConfig) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const updated: FromDef = {
            ...props.modelValue,
            source: {
                ...props.modelValue.source,
                auth,
            },
        }
        emit('update:modelValue', updated)
    }

    function onTypeChange(newType: 'format' | 'entity' | 'previous_step' | 'trigger') {
        let newFrom: FromDef
        if (newType === 'format') {
            newFrom = {
                type: 'format',
                source: {
                    source_type: 'uri',
                    config: { uri: '' },
                    auth: { type: 'none' },
                },
                format: {
                    format_type: 'csv',
                    options: defaultCsvOptions(),
                },
                mapping: {},
            }
        } else if (newType === 'entity') {
            newFrom = {
                type: 'entity',
                entity_definition: '',
                filter: { field: '', operator: '=', value: '' },
                mapping: {},
            }
        } else if (newType === 'trigger') {
            newFrom = {
                type: 'trigger',
                mapping: {},
            }
        } else {
            // previous_step
            newFrom = {
                type: 'previous_step',
                mapping: {},
            }
        }
        emit('update:modelValue', newFrom)
    }

    function addMapping() {
        // Add empty pair directly to MappingEditor's local state
        if (mappingEditorRef.value) {
            mappingEditorRef.value.addEmptyPair()
        } else {
            // Fallback: add to mapping object
            const updated: FromDef = {
                ...props.modelValue,
                mapping: {
                    ...props.modelValue.mapping,
                    '': '',
                },
            }
            emit('update:modelValue', updated)
        }
    }

    function parseCsvHeader(
        text: string,
        delimiter: string | undefined,
        quote: string | undefined
    ): string[] {
        const del = delimiter?.length ? delimiter : ','
        const q = quote?.length ? quote : '"'
        const line = text.split(/\r?\n/)[0] ?? ''
        const cols: string[] = []
        let cur = ''
        let inQuotes = false
        for (let i = 0; i < line.length; i++) {
            const ch = line[i]
            if (ch === q) {
                inQuotes = !inQuotes
                continue
            }
            if (!inQuotes && ch === del) {
                cols.push(cur)
                cur = ''
            } else {
                cur += ch
            }
        }
        cols.push(cur)
        return cols.map(c => c.trim())
    }

    async function onTestUpload(e: Event) {
        const input = e.target as HTMLInputElement | null
        const file = input?.files?.[0]
        if (!file || props.modelValue.type !== 'format') {
            return
        }
        const text = await file.text()
        if (props.modelValue.format.format_type !== 'csv') {
            return
        }
        const opts = props.modelValue.format.options ?? {}
        const header = opts.has_header !== false
        const delimiter = (opts.delimiter as string) || ','
        const quote = (opts.quote as string) || '"'
        let fields: string[]
        if (header) {
            fields = parseCsvHeader(text, delimiter, quote)
        } else {
            const firstLine = text.split(/\r?\n/)[0] ?? ''
            const count = firstLine.split(delimiter).length
            fields = Array.from({ length: count }, (_, i) => `col_${i + 1}`)
        }
        const mapping: Record<string, string> = {}
        for (const f of fields) {
            if (f) {
                mapping[f] = f
            }
        }
        updateField('mapping', mapping)
    }

    async function autoMapFromUri() {
        if (props.modelValue.type !== 'format' || props.modelValue.format.format_type !== 'csv') {
            return
        }
        const config = props.modelValue.source.config
        const uri = typeof config === 'object' && 'uri' in config ? String(config.uri) : ''
        if (!uri) {
            return
        }
        try {
            const res = await fetch(uri)
            const txt = await res.text()
            const autoOpts = props.modelValue.format.options ?? {}
            const header = autoOpts.has_header !== false
            const delimiter = (autoOpts.delimiter as string) || ','
            const quote = (autoOpts.quote as string) || '"'
            let fields: string[]
            if (header) {
                fields = parseCsvHeader(txt, delimiter, quote)
            } else {
                const firstLine = txt.split(/\r?\n/)[0] ?? ''
                const count = firstLine.split(delimiter).length
                fields = Array.from({ length: count }, (_, i) => `col_${i + 1}`)
            }
            const mapping: Record<string, string> = {}
            for (const f of fields) {
                if (f) {
                    mapping[f] = f
                }
            }
            updateField('mapping', mapping)
        } catch {
            // ignore fetch errors (CORS etc.)
        }
    }
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
</style>
