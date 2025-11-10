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
        <template v-if="modelValue.type === 'csv'">
            <div class="d-flex ga-2 mb-2 flex-wrap">
                <v-text-field
                    :model-value="(modelValue as any).uri"
                    :label="t('workflows.dsl.uri')"
                    density="comfortable"
                    @update:model-value="updateField('uri', $event)"
                />
                <CsvOptionsEditor
                    :model-value="(modelValue as any).options"
                    @update:model-value="updateField('options', $event)"
                />
            </div>
            <div class="d-flex align-center ga-2 mb-2 flex-wrap">
                <input
                    type="file"
                    accept=".csv,text/csv"
                    @change="onTestUpload"
                />
                <v-btn
                    size="x-small"
                    variant="tonal"
                    @click="autoMapFromUri"
                    >{{ t('workflows.dsl.auto_map_from_uri') }}</v-btn
                >
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
        <template v-else-if="modelValue.type === 'json'">
            <v-text-field
                :model-value="(modelValue as any).uri"
                :label="t('workflows.dsl.uri')"
                density="comfortable"
                @update:model-value="updateField('uri', $event)"
            />
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
import type { FromDef } from './dsl-utils'
import { defaultCsvOptions } from './dsl-utils'
import CsvOptionsEditor from './CsvOptionsEditor.vue'
import MappingEditor from './MappingEditor.vue'

const props = defineProps<{
    modelValue: FromDef
}>()

const emit = defineEmits<{ (e: 'update:modelValue', value: FromDef): void }>()

const { t } = useTranslations()

const mappingEditorRef = ref<{ addEmptyPair: () => void } | null>(null)

const fromTypes = [
    { title: 'CSV', value: 'csv' },
    { title: 'JSON', value: 'json' },
    { title: 'Entity', value: 'entity' },
]

function updateField(field: string, value: any) {
    const updated: any = { ...props.modelValue }
    updated[field] = value
    // Ensure CSV options exist if type is csv
    if (updated.type === 'csv' && !updated.options) {
        updated.options = defaultCsvOptions()
    }
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

function onTypeChange(newType: 'csv' | 'json' | 'entity') {
    let newFrom: FromDef
    if (newType === 'csv') {
        newFrom = {
            type: 'csv',
            uri: '',
            options: defaultCsvOptions(),
            mapping: {},
        }
    } else if (newType === 'json') {
        newFrom = {
            type: 'json',
            uri: '',
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
    if (!file || props.modelValue.type !== 'csv') {
        return
    }
    const text = await file.text()
    const csvFrom = props.modelValue as any
    const header = csvFrom.options?.header !== false
    const delimiter = csvFrom.options?.delimiter || ','
    const quote = csvFrom.options?.quote || '"'
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
    if (props.modelValue.type !== 'csv') {
        return
    }
    const uri = (props.modelValue as any).uri
    if (!uri) {
        return
    }
    try {
        const res = await fetch(uri)
        const txt = await res.text()
        const csvFrom = props.modelValue as any
        const header = csvFrom.options?.header !== false
        const delimiter = csvFrom.options?.delimiter || ','
        const quote = csvFrom.options?.quote || '"'
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

