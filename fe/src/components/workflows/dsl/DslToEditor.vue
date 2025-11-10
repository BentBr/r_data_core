<template>
    <div>
        <div class="text-caption mb-1">{{ t('workflows.dsl.to') }}</div>
        <v-select
            :model-value="modelValue.type"
            :items="toTypes"
            :label="t('workflows.dsl.to_type')"
            density="comfortable"
            class="mb-2"
            @update:model-value="onTypeChange"
        />
        <template v-if="modelValue.type === 'csv'">
            <div class="d-flex ga-2 mb-2 flex-wrap">
                <v-select
                    :model-value="(modelValue as any).output"
                    :items="outputs"
                    :label="t('workflows.dsl.output')"
                    density="comfortable"
                    @update:model-value="updateField('output', $event)"
                />
                <CsvOptionsEditor
                    :model-value="(modelValue as any).options"
                    @update:model-value="updateField('options', $event)"
                />
            </div>
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_normalized_destination') }}
            </div>
            <MappingEditor
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.normalized')"
                :right-label="t('workflows.dsl.destination')"
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
            <v-select
                :model-value="(modelValue as any).output"
                :items="outputs"
                :label="t('workflows.dsl.output')"
                density="comfortable"
                @update:model-value="updateField('output', $event)"
            />
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_normalized_destination') }}
            </div>
            <MappingEditor
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.normalized')"
                :right-label="t('workflows.dsl.destination')"
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
            <v-select
                :model-value="(modelValue as any).entity_definition"
                :items="entityDefItems"
                item-title="title"
                item-value="value"
                :label="t('workflows.dsl.entity_definition')"
                density="comfortable"
                @update:model-value="onEntityDefChange"
            />
            <v-text-field
                :model-value="(modelValue as any).path"
                :label="t('workflows.dsl.path')"
                density="comfortable"
                @update:model-value="updateField('path', $event)"
            />
            <v-select
                :model-value="(modelValue as any).mode"
                :items="entityModes"
                :label="t('workflows.dsl.mode')"
                density="comfortable"
                @update:model-value="updateField('mode', $event)"
            />
            <v-text-field
                v-if="(modelValue as any).mode === 'update'"
                :model-value="(modelValue as any).update_key"
                :label="t('workflows.dsl.update_key')"
                density="comfortable"
                @update:model-value="updateField('update_key', $event)"
            />
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_normalized_destination') }}
            </div>
            <MappingEditor
                ref="mappingEditorRef"
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.entity_field')"
                :right-label="t('workflows.dsl.normalized_field')"
                :left-items="entityTargetFields"
                :use-select-for-left="true"
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
import { ref, watch, onMounted } from 'vue'
import { useTranslations } from '@/composables/useTranslations'
import { useEntityDefinitions } from '@/composables/useEntityDefinitions'
import { typedHttpClient } from '@/api/typed-client'
import type { ToDef } from './dsl-utils'
import { defaultCsvOptions } from './dsl-utils'
import CsvOptionsEditor from './CsvOptionsEditor.vue'
import MappingEditor from './MappingEditor.vue'

const props = defineProps<{
    modelValue: ToDef
}>()

const emit = defineEmits<{ (e: 'update:modelValue', value: ToDef): void }>()

const { t } = useTranslations()
const { entityDefinitions, loadEntityDefinitions } = useEntityDefinitions()

const entityDefItems = ref<{ title: string; value: string }[]>([])
const entityTargetFields = ref<string[]>([])
const mappingEditorRef = ref<{ addEmptyPair: () => void } | null>(null)

const toTypes = [
    { title: 'CSV', value: 'csv' },
    { title: 'JSON', value: 'json' },
    { title: 'Entity', value: 'entity' },
]
const outputs = [
    { title: 'API', value: 'api' },
    { title: 'Download', value: 'download' },
]
const entityModes = [
    { title: 'Create', value: 'create' },
    { title: 'Update', value: 'update' },
]

// Load entity definitions on mount
watch(
    () => entityDefinitions.value,
    (defs) => {
        entityDefItems.value = (defs || []).map(d => ({
            title: d.display_name || d.entity_type,
            value: d.entity_type,
        }))
    },
    { immediate: true }
)

// Load entity definitions when component is created
onMounted(() => {
    loadEntityDefinitions()
})

// Load entity fields when entity definition changes
async function onEntityDefChange(entityType: string) {
    updateField('entity_definition', entityType)
    await loadEntityFields(entityType)
}

// Load entity fields helper
async function loadEntityFields(entityType: string | null | undefined) {
    if (!entityType) {
        entityTargetFields.value = []
        return
    }
    try {
        const fields = await typedHttpClient.getEntityFields(entityType)
        // Filter out system fields that shouldn't be manually set
        const systemFields = ['uuid', 'updated_at', 'updated_by', 'created_at', 'created_by', 'version']
        entityTargetFields.value = fields
            .map(f => f.name)
            .filter(name => !systemFields.includes(name))
    } catch (e) {
        entityTargetFields.value = []
    }
}

// Load entity fields when entity definition is set initially
watch(
    () => (props.modelValue as any).entity_definition,
    (entityType) => {
        if (props.modelValue.type === 'entity' && entityType) {
            loadEntityFields(entityType)
        }
    },
    { immediate: true }
)

function updateField(field: string, value: any) {
    const updated: any = { ...props.modelValue }
    updated[field] = value
    // Ensure CSV options exist if type is csv
    if (updated.type === 'csv' && !updated.options) {
        updated.options = defaultCsvOptions()
    }
    // Remove 'output' field if type is entity (it should only exist for csv/json)
    if (updated.type === 'entity' && 'output' in updated) {
        delete updated.output
    }
    emit('update:modelValue', updated as ToDef)
}

function onTypeChange(newType: 'csv' | 'json' | 'entity') {
    let newTo: ToDef
    if (newType === 'csv') {
        newTo = {
            type: 'csv',
            output: 'api',
            options: defaultCsvOptions(),
            mapping: {},
        }
    } else if (newType === 'json') {
        newTo = {
            type: 'json',
            output: 'api',
            mapping: {},
        }
    } else {
        // Entity type - NO output field
        newTo = {
            type: 'entity',
            entity_definition: '',
            path: '',
            mode: 'create',
            mapping: {},
        }
    }
    emit('update:modelValue', newTo)
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
        emit('update:modelValue', updated as ToDef)
    }
}
</script>

<style scoped>
.ga-2 {
    gap: 8px;
}
</style>

