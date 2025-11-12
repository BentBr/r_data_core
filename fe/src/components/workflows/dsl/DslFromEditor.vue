<template>
    <div>
        <div class="text-caption mb-1">{{ t('workflows.dsl.from') }}</div>
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
                    :model-value="(modelValue as any).source?.source_type || 'uri'"
                    :items="sourceTypes"
                    :label="t('workflows.dsl.source_type')"
                    density="comfortable"
                    @update:model-value="updateSourceType($event)"
                />
                <v-select
                    :model-value="(modelValue as any).format?.format_type || 'csv'"
                    :items="formatTypes"
                    :label="t('workflows.dsl.format_type')"
                    density="comfortable"
                    @update:model-value="updateFormatType($event)"
                />
            </div>
            <template v-if="(modelValue as any).source?.source_type === 'uri'">
                <v-text-field
                    :model-value="(modelValue as any).source?.config?.uri || ''"
                    :label="t('workflows.dsl.uri')"
                    density="comfortable"
                    class="mb-2"
                    @update:model-value="updateSourceConfig('uri', $event)"
                />
            </template>
            <template v-else-if="(modelValue as any).source?.source_type === 'api'">
                <v-text-field
                    :model-value="(modelValue as any).source?.config?.endpoint || ''"
                    :label="t('workflows.dsl.endpoint')"
                    density="comfortable"
                    class="mb-2"
                    hint="/api/v1/workflows/{uuid}"
                    @update:model-value="updateSourceConfig('endpoint', $event)"
                />
            </template>
            <div v-if="(modelValue as any).format?.format_type === 'csv'" class="mb-2">
                <CsvOptionsEditor
                    :model-value="(modelValue as any).format?.options || {}"
                    @update:model-value="updateFormatOptions($event)"
                />
            </div>
            <div v-if="(modelValue as any).format?.format_type === 'csv'" class="mb-2">
                <div class="d-flex align-center ga-2 flex-wrap">
                    <input
                        type="file"
                        accept=".csv,text/csv"
                        @change="onTestUpload"
                    />
                    <v-btn
                        v-if="(modelValue as any).source?.source_type === 'uri' && (modelValue as any).source?.config?.uri"
                        size="x-small"
                        variant="tonal"
                        @click="autoMapFromUri"
                        >{{ t('workflows.dsl.auto_map_from_uri') }}</v-btn
                    >
                </div>
            </div>
            <div class="mb-2">
                <v-expansion-panels variant="accordion">
                    <v-expansion-panel>
                        <v-expansion-panel-title>{{ t('workflows.dsl.auth_type') }}</v-expansion-panel-title>
                        <v-expansion-panel-text>
                            <AuthConfigEditor
                                :model-value="(modelValue as any).source?.auth || { type: 'none' }"
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
                >{{ t('workflows.dsl.add_mapping') }}</v-btn
            >
        </template>
        <template v-else-if="modelValue.type === 'entity'">
            <v-text-field
                :model-value="(modelValue as any).entity_definition"
                :label="t('workflows.dsl.entity_definition')"
                density="comfortable"
                @update:model-value="updateField('entity_definition', $event)"
            />
            <div class="d-flex ga-2">
                <v-text-field
                    :model-value="(modelValue as any).filter?.field"
                    :label="t('workflows.dsl.filter_field')"
                    density="comfortable"
                    @update:model-value="updateFilterField('field', $event)"
                />
                <v-text-field
                    :model-value="(modelValue as any).filter?.value"
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
                >{{ t('workflows.dsl.add_mapping') }}</v-btn
            >
        </template>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useTranslations } from '@/composables/useTranslations'
import type { FromDef, AuthConfig } from './dsl-utils'
import { defaultCsvOptions } from './dsl-utils'
import CsvOptionsEditor from './CsvOptionsEditor.vue'
import MappingEditor from './MappingEditor.vue'
import AuthConfigEditor from './AuthConfigEditor.vue'

const props = defineProps<{
    modelValue: FromDef
}>()

const emit = defineEmits<{ (e: 'update:modelValue', value: FromDef): void }>()

const { t } = useTranslations()

const mappingEditorRef = ref<{ addEmptyPair: () => void } | null>(null)

const fromTypes = [
    { title: 'Format (CSV/JSON)', value: 'format' },
    { title: 'Entity', value: 'entity' },
]

function updateField(field: string, value: any) {
    const updated: any = { ...props.modelValue }
    updated[field] = value
    // Ensure entity filter exists if type is entity
    if (updated.type === 'entity') {
        if (!updated.filter) {
            updated.filter = { field: '', value: '' }
        }
        if (!updated.mapping) {
            updated.mapping = {}
        }
    }
    emit('update:modelValue', updated as FromDef)
}

function updateFilterField(field: string, value: any) {
    const updated: any = { ...props.modelValue }
    if (!updated.filter) {
        updated.filter = { field: '', value: '' }
    }
    updated.filter[field] = value
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
    const updated: any = { ...props.modelValue }
    if (!updated.source) {
        updated.source = { source_type: newType, config: {}, auth: { type: 'none' } }
    } else {
        updated.source.source_type = newType
        // Reset config based on source type
        if (newType === 'uri') {
            updated.source.config = { uri: '' }
        } else if (newType === 'api') {
            updated.source.config = { endpoint: '' }
        } else {
            updated.source.config = {}
        }
    }
    emit('update:modelValue', updated as FromDef)
}

function updateFormatType(newType: string) {
    const updated: any = { ...props.modelValue }
    if (!updated.format) {
        updated.format = { format_type: newType, options: {} }
    } else {
        updated.format.format_type = newType
        if (newType === 'csv' && !updated.format.options) {
            updated.format.options = defaultCsvOptions()
        }
    }
    emit('update:modelValue', updated as FromDef)
}

function updateSourceConfig(key: string, value: any) {
    const updated: any = { ...props.modelValue }
    if (!updated.source) {
        updated.source = { source_type: 'uri', config: {}, auth: { type: 'none' } }
    }
    if (!updated.source.config) {
        updated.source.config = {}
    }
    updated.source.config[key] = value
    emit('update:modelValue', updated as FromDef)
}

function updateFormatOptions(options: any) {
    const updated: any = { ...props.modelValue }
    if (!updated.format) {
        updated.format = { format_type: 'csv', options: {} }
    }
    updated.format.options = options
    emit('update:modelValue', updated as FromDef)
}

function updateSourceAuth(auth: AuthConfig) {
    const updated: any = { ...props.modelValue }
    if (!updated.source) {
        updated.source = { source_type: 'uri', config: {}, auth: { type: 'none' } }
    }
    updated.source.auth = auth
    emit('update:modelValue', updated as FromDef)
}

function onTypeChange(newType: 'format' | 'entity') {
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
    } else {
        newFrom = {
            type: 'entity',
            entity_definition: '',
            filter: { field: '', value: '' },
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
        const updated: any = { ...props.modelValue }
        if (!updated.mapping) {
            updated.mapping = {}
        }
        updated.mapping[''] = ''
        emit('update:modelValue', updated as FromDef)
    }
}

function parseCsvHeader(text: string, delimiter: string | undefined, quote: string | undefined): string[] {
    const del = delimiter && delimiter.length ? delimiter : ','
    const q = quote && quote.length ? quote : '"'
    const line = text.split(/\r?\n/)[0] || ''
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
    const formatFrom = props.modelValue as any
    if (formatFrom.format?.format_type !== 'csv') {
        return
    }
    const header = formatFrom.format?.options?.has_header !== false
    const delimiter = formatFrom.format?.options?.delimiter || ','
    const quote = formatFrom.format?.options?.quote || '"'
    let fields: string[]
    if (header) {
        fields = parseCsvHeader(text, delimiter, quote)
    } else {
        const firstLine = text.split(/\r?\n/)[0] || ''
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
    if (props.modelValue.type !== 'format') {
        return
    }
    const formatFrom = props.modelValue as any
    if (formatFrom.format?.format_type !== 'csv') {
        return
    }
    const uri = formatFrom.source?.config?.uri
    if (!uri) {
        return
    }
    try {
        const res = await fetch(uri)
        const txt = await res.text()
        const header = formatFrom.format?.options?.has_header !== false
        const delimiter = formatFrom.format?.options?.delimiter || ','
        const quote = formatFrom.format?.options?.quote || '"'
        let fields: string[]
        if (header) {
            fields = parseCsvHeader(txt, delimiter, quote)
        } else {
            const firstLine = txt.split(/\r?\n/)[0] || ''
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
    } catch (e) {
        // ignore fetch errors (CORS etc.)
    }
}
</script>

<style scoped>
.ga-2 {
    gap: 8px;
}
</style>

