<template>
    <div class="dsl-config">
        <div class="d-flex align-center justify-space-between mb-2">
            <div class="text-subtitle-2">{{ t('workflows.dsl.steps_title') }}</div>
            <v-btn
                size="small"
                variant="outlined"
                color="primary"
                @click="addStep"
                >{{ t('workflows.dsl.add_step') }}</v-btn
            >
        </div>
        <v-alert
            v-if="loadError"
            type="error"
            density="compact"
            class="mb-2"
            >{{ loadError }}</v-alert
        >
        <v-skeleton-loader
            v-if="loading"
            type="list-item-two-line"
            class="mb-2"
        />
        <v-expansion-panels
            v-model="openPanels"
            multiple
        >
            <v-expansion-panel
                v-for="(step, idx) in stepsLocal"
                :key="idx"
            >
                <v-expansion-panel-title>
                    {{ t('workflows.dsl.step_label', { n: String(idx + 1) }) }} — from:
                    {{ step.from?.type || '-' }}, transform: {{ step.transform?.type || '-' }}, to:
                    {{ step.to?.type || '-' }}
                    <v-spacer />
                    <v-btn
                        icon="mdi-delete"
                        size="x-small"
                        variant="text"
                        color="error"
                        @click.stop="removeStep(idx)"
                    />
                </v-expansion-panel-title>
                <v-expansion-panel-text>
                    <div class="mb-4">
                        <div class="text-caption mb-1">{{ t('workflows.dsl.from') }}</div>
                        <v-select
                            v-model="step.from.type"
                            :items="fromTypes"
                            :label="t('workflows.dsl.from_type')"
                            density="comfortable"
                            class="mb-2"
                            @update:model-value="onFromTypeChange(step)"
                        />
                        <template v-if="step.from.type === 'csv'">
                            <div class="d-flex ga-2 mb-2 flex-wrap">
                                <v-text-field
                                    v-model="(step.from as any).uri"
                                    :label="t('workflows.dsl.uri')"
                                    density="comfortable"
                                    @update:model-value="emitChange"
                                />
                                <CsvOptionsEditor v-model="(step.from as any).options" />
                            </div>
                            <div class="d-flex align-center ga-2 mb-2 flex-wrap">
                                <input
                                    type="file"
                                    accept=".csv,text/csv"
                                    @change="onTestUpload($event, step)"
                                />
                                <v-btn
                                    size="x-small"
                                    variant="tonal"
                                    @click="autoMapFromUri(step)"
                                    >{{ t('workflows.dsl.auto_map_from_uri') }}</v-btn
                                >
                            </div>
                            <div class="text-caption mb-1 mt-2">
                                {{ t('workflows.dsl.mapping_source_normalized') }}
                            </div>
                            <MappingTable
                                :pairs="getMappingPairs(step.from.mapping)"
                                :left-label="t('workflows.dsl.source')"
                                :right-label="t('workflows.dsl.normalized')"
                                @update-pair="(i, p) => updateMapping(step.from.mapping, p, i)"
                                @delete-pair="i => deleteMapping(step.from.mapping, i)"
                            />
                            <v-btn
                                size="x-small"
                                variant="tonal"
                                @click="addMapping(step.from.mapping)"
                                >{{ t('workflows.dsl.add_mapping') }}</v-btn
                            >
                        </template>
                        <template v-else-if="step.from.type === 'json'">
                            <v-text-field
                                v-model="(step.from as any).uri"
                                :label="t('workflows.dsl.uri')"
                                density="comfortable"
                                @update:model-value="emitChange"
                            />
                            <div class="text-caption mb-1 mt-2">
                                {{ t('workflows.dsl.mapping_source_normalized') }}
                            </div>
                            <MappingTable
                                :pairs="getMappingPairs(step.from.mapping)"
                                :left-label="t('workflows.dsl.source')"
                                :right-label="t('workflows.dsl.normalized')"
                                @update-pair="(i, p) => updateMapping(step.from.mapping, p, i)"
                                @delete-pair="i => deleteMapping(step.from.mapping, i)"
                            />
                            <v-btn
                                size="x-small"
                                variant="tonal"
                                @click="addMapping(step.from.mapping)"
                                >{{ t('workflows.dsl.add_mapping') }}</v-btn
                            >
                        </template>
                        <template v-else-if="step.from.type === 'entity'">
                            <v-text-field
                                v-model="(step.from as any).entity_definition"
                                :label="t('workflows.dsl.entity_definition')"
                                density="comfortable"
                                @update:model-value="emitChange"
                            />
                            <div class="d-flex ga-2">
                                <v-text-field
                                    v-model="(step.from as any).filter.field"
                                    :label="t('workflows.dsl.filter_field')"
                                    density="comfortable"
                                    @update:model-value="emitChange"
                                />
                                <v-text-field
                                    v-model="(step.from as any).filter.value"
                                    :label="t('workflows.dsl.filter_value')"
                                    density="comfortable"
                                    @update:model-value="emitChange"
                                />
                            </div>
                            <div class="text-caption mb-1 mt-2">
                                {{ t('workflows.dsl.mapping_source_normalized') }}
                            </div>
                            <MappingTable
                                :pairs="getMappingPairs(step.from.mapping)"
                                :left-label="t('workflows.dsl.source')"
                                :right-label="t('workflows.dsl.normalized')"
                                @update-pair="(i, p) => updateMapping(step.from.mapping, p, i)"
                                @delete-pair="i => deleteMapping(step.from.mapping, i)"
                            />
                            <v-btn
                                size="x-small"
                                variant="tonal"
                                @click="addMapping(step.from.mapping)"
                                >{{ t('workflows.dsl.add_mapping') }}</v-btn
                            >
                        </template>
                    </div>

                    <div class="mb-4">
                        <div class="text-caption mb-1">{{ t('workflows.dsl.transform') }}</div>
                        <v-select
                            v-model="transformType"
                            :items="transformTypes"
                            :label="t('workflows.dsl.transform_type')"
                            density="comfortable"
                            class="mb-2"
                            @update:model-value="applyTransformType(step)"
                        />
                        <template v-if="transformType === 'arithmetic'">
                            <div class="d-flex ga-2">
                                <v-text-field
                                    v-model="(step.transform as any).target"
                                    :label="t('workflows.dsl.target')"
                                    density="comfortable"
                                    @update:model-value="emitChange"
                                />
                                <v-select
                                    v-model="(step.transform as any).op"
                                    :items="ops"
                                    :label="t('workflows.dsl.op')"
                                    density="comfortable"
                                    class="flex-0"
                                    style="max-width: 160px"
                                    @update:model-value="emitChange"
                                />
                            </div>
                            <div class="d-flex ga-2">
                                <v-select
                                    v-model="left.kind"
                                    :items="operandKinds"
                                    :label="t('workflows.dsl.left_kind')"
                                    density="comfortable"
                                    class="flex-0"
                                    style="max-width: 180px"
                                    @update:model-value="syncLeft(step)"
                                />
                                <v-text-field
                                    v-if="left.kind === 'field'"
                                    v-model="left.field"
                                    :label="t('workflows.dsl.left_field')"
                                    density="comfortable"
                                    @update:model-value="syncLeft(step)"
                                />
                                <v-text-field
                                    v-else
                                    v-model.number="left.value"
                                    :label="t('workflows.dsl.left_value')"
                                    type="number"
                                    density="comfortable"
                                    @update:model-value="syncLeft(step)"
                                />
                            </div>
                            <div class="d-flex ga-2">
                                <v-select
                                    v-model="right.kind"
                                    :items="operandKinds"
                                    :label="t('workflows.dsl.right_kind')"
                                    density="comfortable"
                                    class="flex-0"
                                    style="max-width: 180px"
                                    @update:model-value="syncRight(step)"
                                />
                                <v-text-field
                                    v-if="right.kind === 'field'"
                                    v-model="right.field"
                                    :label="t('workflows.dsl.right_field')"
                                    density="comfortable"
                                    @update:model-value="syncRight(step)"
                                />
                                <v-text-field
                                    v-else
                                    v-model.number="right.value"
                                    :label="t('workflows.dsl.right_value')"
                                    type="number"
                                    density="comfortable"
                                    @update:model-value="syncRight(step)"
                                />
                            </div>
                        </template>
                        <template v-else-if="transformType === 'concat'">
                            <div class="d-flex ga-2">
                                <v-text-field
                                    v-model="(step.transform as any).target"
                                    :label="t('workflows.dsl.target')"
                                    density="comfortable"
                                    @update:model-value="emitChange"
                                />
                                <v-text-field
                                    v-model="(step.transform as any).separator"
                                    :label="t('workflows.dsl.separator')"
                                    density="comfortable"
                                    @update:model-value="emitChange"
                                />
                            </div>
                            <div class="d-flex ga-2">
                                <v-select
                                    v-model="leftConcat.kind"
                                    :items="stringOperandKinds"
                                    :label="t('workflows.dsl.left_kind')"
                                    density="comfortable"
                                    class="flex-0"
                                    style="max-width: 200px"
                                    @update:model-value="syncLeftConcat(step)"
                                />
                                <v-text-field
                                    v-if="leftConcat.kind === 'field'"
                                    v-model="leftConcat.field"
                                    :label="t('workflows.dsl.left_field')"
                                    density="comfortable"
                                    @update:model-value="syncLeftConcat(step)"
                                />
                                <v-text-field
                                    v-else
                                    v-model="leftConcat.value"
                                    :label="t('workflows.dsl.left_value')"
                                    density="comfortable"
                                    @update:model-value="syncLeftConcat(step)"
                                />
                            </div>
                            <div class="d-flex ga-2">
                                <v-select
                                    v-model="rightConcat.kind"
                                    :items="stringOperandKinds"
                                    :label="t('workflows.dsl.right_kind')"
                                    density="comfortable"
                                    class="flex-0"
                                    style="max-width: 200px"
                                    @update:model-value="syncRightConcat(step)"
                                />
                                <v-text-field
                                    v-if="rightConcat.kind === 'field'"
                                    v-model="rightConcat.field"
                                    :label="t('workflows.dsl.right_field')"
                                    density="comfortable"
                                    @update:model-value="syncRightConcat(step)"
                                />
                                <v-text-field
                                    v-else
                                    v-model="rightConcat.value"
                                    :label="t('workflows.dsl.right_value')"
                                    density="comfortable"
                                    @update:model-value="syncRightConcat(step)"
                                />
                            </div>
                        </template>
                    </div>

                    <div>
                        <div class="text-caption mb-1">{{ t('workflows.dsl.to') }}</div>
                        <v-select
                            v-model="step.to.type"
                            :items="toTypes"
                            :label="t('workflows.dsl.to_type')"
                            density="comfortable"
                            class="mb-2"
                            @update:model-value="onToTypeChange(step)"
                        />
                        <template v-if="step.to.type === 'csv'">
                            <div class="d-flex ga-2 mb-2 flex-wrap">
                                <v-select
                                    v-model="(step.to as any).output"
                                    :items="outputs"
                                    :label="t('workflows.dsl.output')"
                                    density="comfortable"
                                    @update:model-value="emitChange"
                                />
                                <CsvOptionsEditor v-model="(step.to as any).options" />
                            </div>
                            <div class="text-caption mb-1 mt-2">
                                {{ t('workflows.dsl.mapping_normalized_destination') }}
                            </div>
                            <MappingTable
                                :pairs="getMappingPairs(step.to.mapping)"
                                :left-label="t('workflows.dsl.normalized')"
                                :right-label="t('workflows.dsl.destination')"
                                @update-pair="(i, p) => updateMapping(step.to.mapping, p, i)"
                                @delete-pair="i => deleteMapping(step.to.mapping, i)"
                            />
                            <v-btn
                                size="x-small"
                                variant="tonal"
                                @click="addMapping(step.to.mapping)"
                                >{{ t('workflows.dsl.add_mapping') }}</v-btn
                            >
                        </template>
                        <template v-else-if="step.to.type === 'json'">
                            <v-select
                                v-model="(step.to as any).output"
                                :items="outputs"
                                :label="t('workflows.dsl.output')"
                                density="comfortable"
                                @update:model-value="emitChange"
                            />
                            <div class="text-caption mb-1 mt-2">
                                {{ t('workflows.dsl.mapping_normalized_destination') }}
                            </div>
                            <div class="mapping-table-wrapper">
                                <v-table
                                    density="comfortable"
                                    class="mapping-table"
                                >
                                    <thead>
                                        <tr>
                                            <th style="width: 45%">
                                                {{ t('workflows.dsl.normalized') }}
                                            </th>
                                            <th style="width: 45%">
                                                {{ t('workflows.dsl.destination') }}
                                            </th>
                                            <th style="width: 10%"></th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr
                                            v-for="(pair, mi) in getMappingPairs(step.to.mapping)"
                                            :key="mi"
                                        >
                                            <td>
                                                <v-text-field
                                                    v-model="pair.k"
                                                    density="comfortable"
                                                    variant="underlined"
                                                    @update:model-value="
                                                        updateMapping(step.to.mapping, pair, mi)
                                                    "
                                                />
                                            </td>
                                            <td>
                                                <v-text-field
                                                    v-model="pair.v"
                                                    density="comfortable"
                                                    variant="underlined"
                                                    @update:model-value="
                                                        updateMapping(step.to.mapping, pair, mi)
                                                    "
                                                />
                                            </td>
                                            <td class="text-right">
                                                <v-btn
                                                    icon="mdi-delete"
                                                    size="x-small"
                                                    variant="text"
                                                    color="error"
                                                    @click="deleteMapping(step.to.mapping, mi)"
                                                />
                                            </td>
                                        </tr>
                                    </tbody>
                                </v-table>
                            </div>
                            <v-btn
                                size="x-small"
                                variant="tonal"
                                @click="addMapping(step.to.mapping)"
                                >{{ t('workflows.dsl.add_mapping') }}</v-btn
                            >
                        </template>
                        <template v-else-if="step.to.type === 'entity'">
                            <v-select
                                v-model="(step.to as any).entity_definition"
                                :items="entityDefItems"
                                item-title="title"
                                item-value="value"
                                :label="t('workflows.dsl.entity_definition')"
                                density="comfortable"
                                @update:model-value="onEntityDefChange"
                            />
                            <v-text-field
                                v-model="(step.to as any).path"
                                :label="t('workflows.dsl.path')"
                                density="comfortable"
                                @update:model-value="emitChange"
                            />
                            <v-select
                                v-model="(step.to as any).mode"
                                :items="entityModes"
                                :label="t('workflows.dsl.mode')"
                                density="comfortable"
                                @update:model-value="emitChange"
                            />
                            <v-text-field
                                v-if="(step.to as any).mode === 'update'"
                                v-model="(step.to as any).update_key"
                                :label="t('workflows.dsl.update_key')"
                                density="comfortable"
                                @update:model-value="emitChange"
                            />
                            <div class="text-caption mb-1 mt-2">
                                {{ t('workflows.dsl.mapping_normalized_destination') }}
                            </div>
                            <MappingTable
                                :pairs="getMappingPairs(step.to.mapping)"
                                :left-label="t('workflows.dsl.normalized')"
                                :right-label="t('workflows.dsl.destination')"
                                :right-items="entityTargetFields"
                                :use-select-for-right="true"
                                @update-pair="(i, p) => updateMapping(step.to.mapping, p, i)"
                                @delete-pair="i => deleteMapping(step.to.mapping, i)"
                            />
                            <v-btn
                                size="x-small"
                                variant="tonal"
                                @click="addMapping(step.to.mapping)"
                                >{{ t('workflows.dsl.add_mapping') }}</v-btn
                            >
                        </template>
                    </div>
                </v-expansion-panel-text>
            </v-expansion-panel>
        </v-expansion-panels>
    </div>
</template>

<script setup lang="ts">
    import { onMounted, ref, watch } from 'vue'
    import { typedHttpClient } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import { useEntityDefinitions } from '@/composables/useEntityDefinitions'
    import CsvOptionsEditor from './dsl/CsvOptionsEditor.vue'
    import MappingTable from './dsl/MappingTable.vue'

    type Mapping = Record<string, string>
    type CsvOptions = { header?: boolean; delimiter?: string; escape?: string; quote?: string }
    type FromDef =
        | { type: 'csv'; uri: string; options: CsvOptions; mapping: Mapping }
        | { type: 'json'; uri: string; mapping: Mapping }
        | {
              type: 'entity'
              entity_definition: string
              filter: { field: string; value: string }
              mapping: Mapping
          }
    type OperandField = { kind: 'field'; field: string }
    type OperandConst = { kind: 'const'; value: number }
    type Operand = OperandField | OperandConst
    type StringOperandField = { kind: 'field'; field: string }
    type StringOperandConst = { kind: 'const_string'; value: string }
    type StringOperand = StringOperandField | StringOperandConst
    type Transform =
        | { type: 'none' }
        | {
              type: 'arithmetic'
              target: string
              left: Operand
              op: 'add' | 'sub' | 'mul' | 'div'
              right: Operand
          }
        | {
              type: 'concat'
              target: string
              left: StringOperand
              separator?: string
              right: StringOperand
          }
    type ToDef =
        | { type: 'csv'; output: 'api' | 'download'; options: CsvOptions; mapping: Mapping }
        | { type: 'json'; output: 'api' | 'download'; mapping: Mapping }
        | {
              type: 'entity'
              entity_definition: string
              path: string
              mode: 'create' | 'update'
              update_key?: string
              identify?: { field: string; value: string }
              mapping: Mapping
          }
    type DslStep = { from: FromDef; transform: Transform; to: ToDef }

    const props = defineProps<{ modelValue: DslStep[] }>()
    const emit = defineEmits<{ (e: 'update:modelValue', value: DslStep[]): void }>()

    const loading = ref(false)
    const loadError = ref<string | null>(null)
    const stepsLocal = ref<DslStep[]>([])
    const openPanels = ref<number[]>([])
    const { t } = useTranslations()

    const { entityDefinitions, loadEntityDefinitions } = useEntityDefinitions()
    const entityDefItems = ref<{ title: string; value: string }[]>([])
    const entityTargetFields = ref<string[]>([])

    const fromTypes = [
        { title: 'CSV', value: 'csv' },
        { title: 'JSON', value: 'json' },
        { title: 'Entity', value: 'entity' },
    ]
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
    const ops = ['add', 'sub', 'mul', 'div']
    const operandKinds = ['field', 'const']
    const stringOperandKinds = ['field', 'const_string']
    const transformTypes = [
        { title: 'None', value: 'none' },
        { title: 'Arithmetic', value: 'arithmetic' },
        { title: 'Concat', value: 'concat' },
    ]
    // Local operand editors (bound to current open panel only)
    const left = ref<Operand>({ kind: 'field', field: '' })
    const right = ref<Operand>({ kind: 'const', value: 0 })
    const leftConcat = ref<StringOperand>({ kind: 'field', field: '' })
    const rightConcat = ref<StringOperand>({ kind: 'const_string', value: '' })
    const transformType = ref<'none' | 'arithmetic' | 'concat'>('none')

    function defaultCsvOptions(): CsvOptions {
        return { header: true, delimiter: ',', escape: undefined, quote: undefined }
    }

    function ensureCsvOptions(step: DslStep) {
        if (step.from?.type === 'csv') {
            const f: any = step.from
            if (!f.options) {
                f.options = defaultCsvOptions()
            }
        }
        if (step.to?.type === 'csv') {
            const t: any = step.to
            if (!t.options) {
                t.options = defaultCsvOptions()
            }
        }
    }

    function ensureEntityFilter(step: DslStep) {
        if (step.from?.type === 'entity') {
            const f: any = step.from
            if (!f.filter) {
                f.filter = { field: '', value: '' }
            }
            if (!f.mapping) {
                f.mapping = {}
            }
        }
    }

    function defaultStep(): DslStep {
        const s: DslStep = {
            from: { type: 'csv', uri: '', options: defaultCsvOptions(), mapping: {} },
            transform: { type: 'none' },
            to: { type: 'json', output: 'api', mapping: {} },
        }
        return s
    }

    const emitScheduled = ref(false)
    function scheduleEmitChange() {
        if (emitScheduled.value) {
            return
        }
        emitScheduled.value = true
        queueMicrotask(() => {
            // Before emitting, ensure options exist where needed
            stepsLocal.value.forEach(s => {
                ensureCsvOptions(s)
                ensureEntityFilter(s)
            })
            emit('update:modelValue', stepsLocal.value)
            emitScheduled.value = false
        })
    }
    function emitChange() {
        scheduleEmitChange()
    }

    function onFromTypeChange(step: DslStep) {
        ensureCsvOptions(step)
        ensureEntityFilter(step)
        scheduleEmitChange()
    }

    function onToTypeChange(step: DslStep) {
        ensureCsvOptions(step)
        scheduleEmitChange()
    }

    async function onEntityDefChange(entityType: string) {
        // do not emit immediately to avoid recursive updates during model change
        scheduleEmitChange()
        if (!entityType) {
            entityTargetFields.value = []
            return
        }
        try {
            const fields = await typedHttpClient.getEntityFields(entityType)
            entityTargetFields.value = fields.map(f => f.name)
        } catch (e) {
            entityTargetFields.value = []
        }
    }

    function addStep() {
        stepsLocal.value.push(defaultStep())
        openPanels.value = [stepsLocal.value.length - 1]
        scheduleEmitChange()
    }
    function removeStep(idx: number) {
        stepsLocal.value.splice(idx, 1)
        scheduleEmitChange()
    }

    function getMappingPairs(mapping: Mapping) {
        const entries = Object.entries(mapping)
        return entries.map(([k, v]) => ({ k, v }))
    }
    function updateMapping(mapping: Mapping, pair: { k: string; v: string }, idx: number) {
        // rebuild mapping from pairs
        const pairs = getMappingPairs(mapping)
        pairs[idx] = pair
        const out: Mapping = {}
        for (const p of pairs) {
            if (p.k && p.v) {
                out[p.k] = p.v
            }
        }
        Object.keys(mapping).forEach(k => delete (mapping as any)[k])
        Object.assign(mapping, out)
        scheduleEmitChange()
    }
    function addMapping(mapping: Mapping) {
        // add empty placeholder pair; will be ignored until filled
        mapping[''] = ''
        scheduleEmitChange()
    }
    function deleteMapping(mapping: Mapping, idx: number) {
        const pairs = getMappingPairs(mapping)
        const toDelete = pairs[idx]
        if (toDelete?.k) {
            delete mapping[toDelete.k]
        } else {
            delete mapping['']
        }
        scheduleEmitChange()
    }

    function syncLeft(step: DslStep) {
        ;(step.transform as any).left = left.value
        emitChange()
    }
    function syncRight(step: DslStep) {
        ;(step.transform as any).right = right.value
        emitChange()
    }
    function syncLeftConcat(step: DslStep) {
        if ((step.transform as any).type !== 'concat') {
            return
        }
        ;(step.transform as any).left = leftConcat.value
        emitChange()
    }
    function syncRightConcat(step: DslStep) {
        if ((step.transform as any).type !== 'concat') {
            return
        }
        ;(step.transform as any).right = rightConcat.value
        emitChange()
    }
    function applyTransformType(step: DslStep) {
        if (transformType.value === 'none') {
            step.transform = { type: 'none' }
        } else if (transformType.value === 'arithmetic') {
            step.transform = {
                type: 'arithmetic',
                target: '',
                left: { kind: 'field', field: '' },
                op: 'add',
                right: { kind: 'const', value: 0 },
            }
        } else {
            step.transform = {
                type: 'concat',
                target: '',
                left: { kind: 'field', field: '' },
                separator: ' ',
                right: { kind: 'field', field: '' },
            }
        }
        emitChange()
    }

    function parseCsvHeader(
        text: string,
        delimiter: string | undefined,
        quote: string | undefined
    ): string[] {
        const del = delimiter?.length ? delimiter : ','
        const q = quote?.length ? quote : '"'
        // naive split respecting quotes for header row only
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

    async function onTestUpload(e: Event, step: DslStep) {
        const input = e.target as HTMLInputElement | null
        const file = input?.files?.[0]
        if (!file) {
            return
        }
        const text = await file.text()
        ensureCsvOptions(step)
        const header = (step.from as any).options?.header !== false
        const delimiter = (step.from as any).options?.delimiter || ','
        const quote = (step.from as any).options?.quote || '"'
        let fields: string[]
        if (header) {
            fields = parseCsvHeader(text, delimiter, quote)
        } else {
            const firstLine = text.split(/\r?\n/)[0] || ''
            const count = firstLine.split(delimiter).length
            fields = Array.from({ length: count }, (_, i) => `col_${i + 1}`)
        }
        // Auto mapping source->normalized as identity
        const mapping: Mapping = {}
        for (const f of fields) {
            if (f) {
                mapping[f] = f
            }
        }
        ;(step.from as any).mapping = mapping
        emitChange()
    }

    async function autoMapFromUri(step: DslStep) {
        const uri = (step.from as any).uri
        if (!uri) {
            return
        }
        try {
            const res = await fetch(uri)
            const txt = await res.text()
            ensureCsvOptions(step)
            const header = (step.from as any).options?.header !== false
            const delimiter = (step.from as any).options?.delimiter || ','
            const quote = (step.from as any).options?.quote || '"'
            let fields: string[]
            if (header) {
                fields = parseCsvHeader(txt, delimiter, quote)
            } else {
                const firstLine = txt.split(/\r?\n/)[0] || ''
                const count = firstLine.split(delimiter).length
                fields = Array.from({ length: count }, (_, i) => `col_${i + 1}`)
            }
            const mapping: Mapping = {}
            for (const f of fields) {
                if (f) {
                    mapping[f] = f
                }
            }
            ;(step.from as any).mapping = mapping
            emitChange()
        } catch (e) {
            // ignore fetch errors (CORS etc.)
        }
    }

    onMounted(async () => {
        loading.value = true
        try {
            await Promise.all([
                typedHttpClient.getDslFromOptions(),
                typedHttpClient.getDslToOptions(),
                typedHttpClient.getDslTransformOptions(),
                loadEntityDefinitions(),
            ])
            entityDefItems.value = (entityDefinitions.value || []).map(d => ({
                title: d.display_name || d.entity_type,
                value: d.entity_type,
            }))
        } catch (e: any) {
            loadError.value = e?.message || 'Failed to load DSL options'
        } finally {
            loading.value = false
        }
    })

    watch(
        () => props.modelValue,
        v => {
            stepsLocal.value = Array.isArray(v) ? JSON.parse(JSON.stringify(v)) : []
            // ensure options and entity filter on all steps
            stepsLocal.value.forEach(s => {
                ensureCsvOptions(s)
                ensureEntityFilter(s)
            })
            const first = stepsLocal.value[openPanels.value?.[0] ?? 0]
            if (first?.transform?.type) {
                transformType.value = first.transform.type as any
            } else {
                transformType.value = 'none'
            }
        },
        { immediate: true }
    )
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
    .dsl-config {
        width: 100%;
    }
    .mapping-table-wrapper {
        overflow-x: auto;
    }
    .mapping-table {
        min-width: 560px;
    }
</style>
