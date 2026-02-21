<template>
    <div>
        <div class="text-subtitle-2 font-weight-bold mb-2">{{ t('workflows.dsl.transform') }}</div>
        <v-select
            :model-value="transformType"
            :items="transformTypes"
            :label="t('workflows.dsl.transform_type')"
            density="comfortable"
            class="mb-2"
            @update:model-value="onTypeChange"
        />
        <template v-if="transformType === 'arithmetic'">
            <div class="d-flex ga-2">
                <v-text-field
                    :model-value="arithmeticTarget"
                    :label="t('workflows.dsl.target')"
                    density="comfortable"
                    @update:model-value="updateField('target', $event)"
                />
                <v-select
                    :model-value="arithmeticOp"
                    :items="ops"
                    item-title="title"
                    item-value="value"
                    :label="t('workflows.dsl.op')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 160px"
                    @update:model-value="updateField('op', $event)"
                >
                    <template #item="{ item, props: itemProps }">
                        <v-list-item
                            v-bind="itemProps"
                            :title="item.raw?.title || item.title"
                        >
                            <template #prepend>
                                <SmartIcon
                                    :icon="item.raw?.icon || 'file-text'"
                                    size="sm"
                                    class="mr-2"
                                />
                            </template>
                        </v-list-item>
                    </template>
                    <template #selection="{ item }">
                        <SmartIcon
                            :icon="item.raw?.icon || 'file-text'"
                            size="sm"
                            class="mr-1"
                        />
                        <span>{{ item.raw?.title || item.title }}</span>
                    </template>
                </v-select>
            </div>
            <div class="d-flex ga-2">
                <v-select
                    :model-value="left.kind"
                    :items="operandKinds"
                    :label="t('workflows.dsl.left_kind')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 180px"
                    @update:model-value="updateLeftKind"
                />
                <v-select
                    v-if="left.kind === 'field' && availableFields.length > 0"
                    :model-value="left.field"
                    :items="availableFields"
                    :label="t('workflows.dsl.left_field')"
                    density="comfortable"
                    @update:model-value="updateLeftField"
                />
                <v-text-field
                    v-else-if="left.kind === 'field'"
                    :model-value="left.field"
                    :label="t('workflows.dsl.left_field')"
                    density="comfortable"
                    @update:model-value="updateLeftField"
                />
                <v-text-field
                    v-else-if="left.kind === 'const'"
                    :model-value="String(left.value)"
                    :label="t('workflows.dsl.left_value')"
                    type="number"
                    density="comfortable"
                    @update:model-value="updateLeftValue(Number($event))"
                />
            </div>
            <div class="d-flex ga-2">
                <v-select
                    :model-value="right.kind"
                    :items="operandKinds"
                    :label="t('workflows.dsl.right_kind')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 180px"
                    @update:model-value="updateRightKind"
                />
                <v-select
                    v-if="right.kind === 'field' && availableFields.length > 0"
                    :model-value="right.field"
                    :items="availableFields"
                    :label="t('workflows.dsl.right_field')"
                    density="comfortable"
                    @update:model-value="updateRightField"
                />
                <v-text-field
                    v-else-if="right.kind === 'field'"
                    :model-value="right.field"
                    :label="t('workflows.dsl.right_field')"
                    density="comfortable"
                    @update:model-value="updateRightField"
                />
                <v-text-field
                    v-else-if="right.kind === 'const'"
                    :model-value="String(right.value)"
                    :label="t('workflows.dsl.right_value')"
                    type="number"
                    density="comfortable"
                    @update:model-value="updateRightValue(Number($event))"
                />
            </div>
        </template>
        <template v-else-if="transformType === 'concat'">
            <div class="d-flex ga-2">
                <v-text-field
                    :model-value="props.modelValue.type === 'concat' ? props.modelValue.target : ''"
                    :label="t('workflows.dsl.target')"
                    density="comfortable"
                    @update:model-value="updateField('target', $event)"
                />
                <v-text-field
                    :model-value="concatSeparator"
                    :label="t('workflows.dsl.separator')"
                    density="comfortable"
                    @update:model-value="updateField('separator', $event)"
                />
            </div>
            <div class="d-flex ga-2">
                <v-select
                    :model-value="leftConcat.kind"
                    :items="stringOperandKinds"
                    :label="t('workflows.dsl.left_kind')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 200px"
                    @update:model-value="updateLeftConcatKind"
                />
                <v-select
                    v-if="leftConcat.kind === 'field' && availableFields.length > 0"
                    :model-value="leftConcat.field"
                    :items="availableFields"
                    :label="t('workflows.dsl.left_field')"
                    density="comfortable"
                    @update:model-value="updateLeftConcatField"
                />
                <v-text-field
                    v-else-if="leftConcat.kind === 'field'"
                    :model-value="leftConcat.field"
                    :label="t('workflows.dsl.left_field')"
                    density="comfortable"
                    @update:model-value="updateLeftConcatField"
                />
                <v-text-field
                    v-else
                    :model-value="leftConcat.value"
                    :label="t('workflows.dsl.left_value')"
                    density="comfortable"
                    @update:model-value="updateLeftConcatValue"
                />
            </div>
            <div class="d-flex ga-2">
                <v-select
                    :model-value="rightConcat.kind"
                    :items="stringOperandKinds"
                    :label="t('workflows.dsl.right_kind')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 200px"
                    @update:model-value="updateRightConcatKind"
                />
                <v-select
                    v-if="rightConcat.kind === 'field' && availableFields.length > 0"
                    :model-value="rightConcat.field"
                    :items="availableFields"
                    :label="t('workflows.dsl.right_field')"
                    density="comfortable"
                    @update:model-value="updateRightConcatField"
                />
                <v-text-field
                    v-else-if="rightConcat.kind === 'field'"
                    :model-value="rightConcat.field"
                    :label="t('workflows.dsl.right_field')"
                    density="comfortable"
                    @update:model-value="updateRightConcatField"
                />
                <v-text-field
                    v-else
                    :model-value="rightConcat.value"
                    :label="t('workflows.dsl.right_value')"
                    density="comfortable"
                    @update:model-value="updateRightConcatValue"
                />
            </div>
        </template>
        <template v-else-if="transformType === 'build_path'">
            <v-text-field
                :model-value="buildPathTarget"
                :label="t('workflows.dsl.target')"
                density="comfortable"
                class="mb-2"
                @update:model-value="updateBuildPathField('target', $event)"
            />
            <v-textarea
                :model-value="buildPathTemplate"
                :label="t('workflows.dsl.path_template')"
                :hint="t('workflows.dsl.path_template_hint')"
                density="comfortable"
                class="mb-2"
                rows="2"
                @update:model-value="updateBuildPathField('template', $event)"
            />
            <v-text-field
                :model-value="buildPathSeparator"
                :label="t('workflows.dsl.separator')"
                density="comfortable"
                class="mb-2"
                @update:model-value="updateBuildPathField('separator', $event)"
            />
            <div class="text-caption mb-1">{{ t('workflows.dsl.field_transforms') }}</div>
            <div class="d-flex ga-2 mb-2">
                <v-btn
                    variant="outlined"
                    size="small"
                    @click="addBuildPathFieldTransform"
                >
                    {{ t('workflows.dsl.add_filter') }}
                </v-btn>
            </div>
            <MappingTable
                :pairs="buildPathFieldTransforms"
                :left-label="t('workflows.dsl.field_name')"
                :right-label="t('workflows.dsl.transform_type')"
                :right-items="['lowercase', 'uppercase', 'trim', 'normalize', 'slug', 'hash']"
                :use-select-for-right="true"
                @update-pair="updateBuildPathFieldTransform"
                @delete-pair="deleteBuildPathFieldTransform"
            />
        </template>
        <template v-else-if="transformType === 'resolve_entity_path'">
            <v-text-field
                :model-value="resolveEntityPathTarget"
                :label="t('workflows.dsl.target_path')"
                density="comfortable"
                class="mb-2"
                @update:model-value="updateResolveEntityPathField('target_path', $event)"
            />
            <v-text-field
                :model-value="resolveEntityPathTargetUuid"
                :label="t('workflows.dsl.target_uuid')"
                density="comfortable"
                class="mb-2"
                hint="Optional: field to store entity UUID (use as parent_uuid for children)"
                persistent-hint
                @update:model-value="updateResolveEntityPathField('target_uuid', $event)"
            />
            <v-text-field
                :model-value="resolveEntityPathEntityType"
                :label="t('workflows.dsl.entity_type')"
                density="comfortable"
                class="mb-2"
                required
                @update:model-value="updateResolveEntityPathField('entity_type', $event)"
            />
            <div class="text-caption mb-1">{{ t('workflows.dsl.filters') }}</div>
            <div
                v-for="(filter, field) in resolveEntityPathFilters"
                :key="field"
                class="d-flex ga-2 mb-2"
            >
                <v-text-field
                    :model-value="field"
                    :label="t('workflows.dsl.filter_field')"
                    density="comfortable"
                    readonly
                    class="flex-0"
                    style="max-width: 200px"
                />
                <v-select
                    :model-value="filter.kind"
                    :items="stringOperandKinds"
                    :label="t('workflows.dsl.value_kind')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 180px"
                    @update:model-value="updateFilterKind(field, $event)"
                />
                <v-select
                    v-if="filter.kind === 'field' && availableFields.length > 0"
                    :model-value="filter.field"
                    :items="availableFields"
                    :label="t('workflows.dsl.value')"
                    density="comfortable"
                    @update:model-value="updateFilterField(field, $event)"
                />
                <v-text-field
                    v-else-if="filter.kind === 'field'"
                    :model-value="filter.field"
                    :label="t('workflows.dsl.value')"
                    density="comfortable"
                    @update:model-value="updateFilterField(field, $event)"
                />
                <v-text-field
                    v-else
                    :model-value="filter.value"
                    :label="t('workflows.dsl.value')"
                    density="comfortable"
                    @update:model-value="updateFilterValue(field, $event)"
                />
                <v-btn
                    icon="mdi-delete"
                    variant="text"
                    size="small"
                    @click="removeFilter(field)"
                />
            </div>
            <v-btn
                variant="outlined"
                size="small"
                class="mb-2"
                @click="addFilter"
            >
                {{ t('workflows.dsl.add_filter') }}
            </v-btn>
            <v-text-field
                :model-value="resolveEntityPathFallback"
                :label="t('workflows.dsl.fallback_path')"
                density="comfortable"
                class="mb-2"
                hint="Optional: path to use if entity not found"
                persistent-hint
                @update:model-value="updateResolveEntityPathField('fallback_path', $event)"
            />
        </template>
        <template v-else-if="transformType === 'get_or_create_entity'">
            <v-text-field
                :model-value="getOrCreateTargetPath"
                :label="t('workflows.dsl.target_path')"
                density="comfortable"
                class="mb-2"
                @update:model-value="updateGetOrCreateField('target_path', $event)"
            />
            <v-text-field
                :model-value="getOrCreateTargetUuid"
                :label="t('workflows.dsl.target_uuid')"
                density="comfortable"
                class="mb-2"
                hint="Optional: field to store entity UUID (use as parent_uuid for children)"
                persistent-hint
                @update:model-value="updateGetOrCreateField('target_uuid', $event)"
            />
            <v-text-field
                :model-value="getOrCreateEntityType"
                :label="t('workflows.dsl.entity_type')"
                density="comfortable"
                class="mb-2"
                required
                @update:model-value="updateGetOrCreateField('entity_type', $event)"
            />
            <v-textarea
                :model-value="getOrCreatePathTemplate"
                :label="t('workflows.dsl.path_template')"
                :hint="t('workflows.dsl.path_template_hint')"
                density="comfortable"
                class="mb-2"
                rows="2"
                @update:model-value="updateGetOrCreateField('path_template', $event)"
            />
            <v-text-field
                :model-value="getOrCreatePathSeparator"
                :label="t('workflows.dsl.path_separator')"
                density="comfortable"
                class="mb-2"
                hint="Optional: separator for path segments (default: /)"
                persistent-hint
                @update:model-value="updateGetOrCreateField('path_separator', $event)"
            />
        </template>
        <AuthenticateTransformEditor
            v-else-if="transformType === 'authenticate'"
            :model-value="props.modelValue"
            :available-fields="availableFields"
            @update:model-value="emit('update:modelValue', $event)"
        />
    </div>
</template>

<script setup lang="ts">
    import { ref, watch, computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import MappingTable from './MappingTable.vue'
    import AuthenticateTransformEditor from './AuthenticateTransformEditor.vue'
    import type { Transform, Operand, StringOperand } from './dsl-utils'

    const props = defineProps<{
        modelValue: Transform
        availableFields?: string[]
    }>()

    const emit = defineEmits<{ (e: 'update:modelValue', value: Transform): void }>()

    const { t } = useTranslations()

    // Use available fields or empty array
    const availableFields = computed(() => props.availableFields ?? [])

    const ops = computed(() => [
        { title: t('workflows.dsl.op_add'), value: 'add', icon: 'plus' },
        { title: t('workflows.dsl.op_sub'), value: 'sub', icon: 'minus' },
        { title: t('workflows.dsl.op_mul'), value: 'mul', icon: 'x' },
        { title: t('workflows.dsl.op_div'), value: 'div', icon: 'divide' },
    ])
    const operandKinds = ['field', 'const']
    const stringOperandKinds = ['field', 'const_string']
    const transformTypes = [
        { title: 'None', value: 'none' },
        { title: 'Arithmetic', value: 'arithmetic' },
        { title: 'Concat', value: 'concat' },
        { title: 'Build Path', value: 'build_path' },
        { title: 'Resolve Entity Path', value: 'resolve_entity_path' },
        { title: 'Get or Create Entity', value: 'get_or_create_entity' },
        { title: 'Authenticate', value: 'authenticate' },
    ]

    const transformType = ref<
        | 'none'
        | 'arithmetic'
        | 'concat'
        | 'build_path'
        | 'resolve_entity_path'
        | 'get_or_create_entity'
        | 'authenticate'
    >(props.modelValue.type)

    // Computed properties to avoid 'as any' in templates
    const arithmeticTarget = computed(() => {
        if (props.modelValue.type === 'arithmetic') {
            return props.modelValue.target
        }
        return ''
    })

    const arithmeticOp = computed(() => {
        if (props.modelValue.type === 'arithmetic') {
            return props.modelValue.op
        }
        return 'add'
    })

    const concatSeparator = computed(() => {
        if (props.modelValue.type === 'concat') {
            return props.modelValue.separator ?? ' '
        }
        return ' '
    })

    // BuildPath computed properties
    const buildPathTarget = computed(() => {
        if (props.modelValue.type === 'build_path') {
            return props.modelValue.target
        }
        return ''
    })

    const buildPathTemplate = computed(() => {
        if (props.modelValue.type === 'build_path') {
            return props.modelValue.template
        }
        return ''
    })

    const buildPathSeparator = computed(() => {
        if (props.modelValue.type === 'build_path') {
            return props.modelValue.separator ?? '/'
        }
        return '/'
    })

    const buildPathFieldTransforms = computed({
        get() {
            if (props.modelValue.type === 'build_path') {
                const transforms = props.modelValue.field_transforms ?? {}
                return Object.entries(transforms).map(([k, v]) => ({ k, v }))
            }
            return []
        },
        set(pairs: Array<{ k: string; v: string }>) {
            if (props.modelValue.type === 'build_path') {
                const transforms: Record<string, string> = {}
                for (const { k, v } of pairs) {
                    if (k && v) {
                        transforms[k] = v
                    }
                }
                const updated: Transform = {
                    ...props.modelValue,
                    field_transforms: Object.keys(transforms).length > 0 ? transforms : undefined,
                }
                emit('update:modelValue', updated)
            }
        },
    })

    // ResolveEntityPath computed properties
    const resolveEntityPathTarget = computed(() => {
        if (props.modelValue.type === 'resolve_entity_path') {
            return props.modelValue.target_path
        }
        return ''
    })

    const resolveEntityPathTargetUuid = computed(() => {
        if (props.modelValue.type === 'resolve_entity_path') {
            return props.modelValue.target_uuid ?? ''
        }
        return ''
    })

    const resolveEntityPathEntityType = computed(() => {
        if (props.modelValue.type === 'resolve_entity_path') {
            return props.modelValue.entity_type
        }
        return ''
    })

    const resolveEntityPathFilters = computed(() => {
        if (props.modelValue.type === 'resolve_entity_path') {
            return props.modelValue.filters
        }
        return {}
    })

    const resolveEntityPathFallback = computed(() => {
        if (props.modelValue.type === 'resolve_entity_path') {
            return props.modelValue.fallback_path ?? ''
        }
        return ''
    })

    // GetOrCreateEntity computed properties
    const getOrCreateTargetPath = computed(() => {
        if (props.modelValue.type === 'get_or_create_entity') {
            return props.modelValue.target_path
        }
        return ''
    })

    const getOrCreateTargetUuid = computed(() => {
        if (props.modelValue.type === 'get_or_create_entity') {
            return props.modelValue.target_uuid ?? ''
        }
        return ''
    })

    const getOrCreateEntityType = computed(() => {
        if (props.modelValue.type === 'get_or_create_entity') {
            return props.modelValue.entity_type
        }
        return ''
    })

    const getOrCreatePathTemplate = computed(() => {
        if (props.modelValue.type === 'get_or_create_entity') {
            return props.modelValue.path_template
        }
        return ''
    })

    const getOrCreatePathSeparator = computed(() => {
        if (props.modelValue.type === 'get_or_create_entity') {
            return props.modelValue.path_separator ?? '/'
        }
        return '/'
    })

    // Local operand editors
    const left = ref<Operand>({ kind: 'field', field: '' })
    const right = ref<Operand>({ kind: 'const', value: 0 })
    const leftConcat = ref<StringOperand>({ kind: 'field', field: '' })
    const rightConcat = ref<StringOperand>({ kind: 'const_string', value: '' })

    // Sync local operands from modelValue
    watch(
        () => props.modelValue,
        newTransform => {
            transformType.value = newTransform.type
            if (newTransform.type === 'arithmetic') {
                left.value = newTransform.left
                right.value = newTransform.right
            } else if (newTransform.type === 'concat') {
                leftConcat.value = newTransform.left
                rightConcat.value = newTransform.right
            }
        },
        { immediate: true, deep: true }
    )

    // Filter management for ResolveEntityPath
    function addFilter() {
        if (props.modelValue.type === 'resolve_entity_path') {
            const filters = { ...props.modelValue.filters }
            filters[''] = { kind: 'field', field: '' }
            const updated: Transform = {
                ...props.modelValue,
                filters,
            }
            emit('update:modelValue', updated)
        }
    }

    function removeFilter(field: string) {
        if (props.modelValue.type === 'resolve_entity_path') {
            const filters = { ...props.modelValue.filters }
            delete filters[field]
            const updated: Transform = {
                ...props.modelValue,
                filters,
            }
            emit('update:modelValue', updated)
        }
    }

    function updateFilterKind(field: string, kind: 'field' | 'const_string') {
        if (props.modelValue.type === 'resolve_entity_path') {
            const filters = { ...props.modelValue.filters }
            if (kind === 'field') {
                filters[field] = { kind: 'field', field: '' }
            } else {
                filters[field] = { kind: 'const_string', value: '' }
            }
            const updated: Transform = {
                ...props.modelValue,
                filters,
            }
            emit('update:modelValue', updated)
        }
    }

    function updateFilterField(field: string, value: string) {
        if (props.modelValue.type === 'resolve_entity_path') {
            const filters = { ...props.modelValue.filters }
            if (filters[field].kind === 'field') {
                filters[field] = { kind: 'field', field: value }
            }
            const updated: Transform = {
                ...props.modelValue,
                filters,
            }
            emit('update:modelValue', updated)
        }
    }

    function updateFilterValue(field: string, value: string) {
        if (props.modelValue.type === 'resolve_entity_path') {
            const filters = { ...props.modelValue.filters }
            if (filters[field].kind === 'const_string') {
                filters[field] = { kind: 'const_string', value }
            }
            const updated: Transform = {
                ...props.modelValue,
                filters,
            }
            emit('update:modelValue', updated)
        }
    }

    function updateField(field: string, value: unknown) {
        if (props.modelValue.type === 'arithmetic') {
            const updated: Transform = {
                ...props.modelValue,
                [field]: value,
            }
            emit('update:modelValue', updated)
        } else if (props.modelValue.type === 'concat') {
            const updated: Transform = {
                ...props.modelValue,
                [field]: value,
            }
            emit('update:modelValue', updated)
        }
    }

    function updateBuildPathField(field: string, value: unknown) {
        if (props.modelValue.type === 'build_path') {
            const updated: Transform = {
                ...props.modelValue,
                [field]: value,
            }
            emit('update:modelValue', updated)
        }
    }

    function updateBuildPathFieldTransform(idx: number, pair: { k: string; v: string }) {
        if (props.modelValue.type === 'build_path') {
            const currentPairs = buildPathFieldTransforms.value.map(p => ({ ...p }))
            currentPairs[idx] = pair
            const transforms: Record<string, string> = {}
            for (const { k, v } of currentPairs) {
                if (k && v) {
                    transforms[k] = v
                }
            }
            const updated: Transform = {
                ...props.modelValue,
                field_transforms: Object.keys(transforms).length > 0 ? transforms : undefined,
            }
            emit('update:modelValue', updated)
        }
    }

    function deleteBuildPathFieldTransform(idx: number) {
        if (props.modelValue.type === 'build_path') {
            const currentPairs = buildPathFieldTransforms.value.map(p => ({ ...p }))
            currentPairs.splice(idx, 1)
            const transforms: Record<string, string> = {}
            for (const { k, v } of currentPairs) {
                if (k && v) {
                    transforms[k] = v
                }
            }
            const updated: Transform = {
                ...props.modelValue,
                field_transforms: Object.keys(transforms).length > 0 ? transforms : undefined,
            }
            emit('update:modelValue', updated)
        }
    }

    function addBuildPathFieldTransform() {
        if (props.modelValue.type === 'build_path') {
            const currentPairs = buildPathFieldTransforms.value.map(p => ({ ...p }))
            currentPairs.push({ k: '', v: '' })
            const transforms: Record<string, string> = {}
            for (const { k, v } of currentPairs) {
                if (k && v) {
                    transforms[k] = v
                }
            }
            const updated: Transform = {
                ...props.modelValue,
                field_transforms: Object.keys(transforms).length > 0 ? transforms : undefined,
            }
            emit('update:modelValue', updated)
        }
    }

    function updateResolveEntityPathField(field: string, value: unknown) {
        if (props.modelValue.type === 'resolve_entity_path') {
            const updated: Transform = {
                ...props.modelValue,
                [field]: value ?? undefined,
            }
            emit('update:modelValue', updated)
        }
    }

    function updateGetOrCreateField(field: string, value: unknown) {
        if (props.modelValue.type === 'get_or_create_entity') {
            const updated: Transform = {
                ...props.modelValue,
                [field]: value ?? undefined,
            }
            emit('update:modelValue', updated)
        }
    }

    function onTypeChange(
        newType:
            | 'none'
            | 'arithmetic'
            | 'concat'
            | 'build_path'
            | 'resolve_entity_path'
            | 'get_or_create_entity'
            | 'authenticate'
    ) {
        transformType.value = newType
        let newTransform: Transform
        if (newType === 'none') {
            newTransform = { type: 'none' }
        } else if (newType === 'arithmetic') {
            newTransform = {
                type: 'arithmetic',
                target: '',
                left: { kind: 'field', field: '' },
                op: 'add',
                right: { kind: 'const', value: 0 },
            }
            left.value = { kind: 'field', field: '' }
            right.value = { kind: 'const', value: 0 }
        } else if (newType === 'concat') {
            newTransform = {
                type: 'concat',
                target: '',
                left: { kind: 'field', field: '' },
                separator: ' ',
                right: { kind: 'field', field: '' },
            }
            leftConcat.value = { kind: 'field', field: '' }
            rightConcat.value = { kind: 'field', field: '' }
        } else if (newType === 'build_path') {
            newTransform = {
                type: 'build_path',
                target: '',
                template: '',
                separator: '/',
            }
        } else if (newType === 'resolve_entity_path') {
            newTransform = {
                type: 'resolve_entity_path',
                target_path: '',
                entity_type: '',
                filters: {},
            }
        } else if (newType === 'get_or_create_entity') {
            newTransform = {
                type: 'get_or_create_entity',
                target_path: '',
                entity_type: '',
                path_template: '',
            }
        } else {
            // newType is 'authenticate' at this point
            newTransform = {
                type: 'authenticate',
                entity_type: '',
                identifier_field: '',
                password_field: '',
                input_identifier: '',
                input_password: '',
                target_token: '',
            }
        }
        emit('update:modelValue', newTransform)
    }

    // Arithmetic operand updates (UI only supports field/const, not external_entity_field)
    function updateLeftKind(kind: 'field' | 'const' | 'external_entity_field') {
        if (kind === 'field' || kind === 'external_entity_field') {
            left.value = { kind: 'field', field: '' }
        } else {
            left.value = { kind: 'const', value: 0 }
        }
        syncLeft()
    }

    function updateLeftField(field: string) {
        left.value = { kind: 'field', field }
        syncLeft()
    }

    function updateLeftValue(value: number) {
        left.value = { kind: 'const', value }
        syncLeft()
    }

    function updateRightKind(kind: 'field' | 'const' | 'external_entity_field') {
        if (kind === 'field' || kind === 'external_entity_field') {
            right.value = { kind: 'field', field: '' }
        } else {
            right.value = { kind: 'const', value: 0 }
        }
        syncRight()
    }

    function updateRightField(field: string) {
        right.value = { kind: 'field', field }
        syncRight()
    }

    function updateRightValue(value: number) {
        right.value = { kind: 'const', value }
        syncRight()
    }

    function syncLeft() {
        if (transformType.value === 'arithmetic' && props.modelValue.type === 'arithmetic') {
            const updated: Transform = {
                ...props.modelValue,
                left: { ...left.value },
            }
            emit('update:modelValue', updated)
        }
    }

    function syncRight() {
        if (transformType.value === 'arithmetic' && props.modelValue.type === 'arithmetic') {
            const updated: Transform = {
                ...props.modelValue,
                right: { ...right.value },
            }
            emit('update:modelValue', updated)
        }
    }

    // Concat operand updates
    function updateLeftConcatKind(kind: 'field' | 'const_string') {
        if (kind === 'field') {
            leftConcat.value = { kind: 'field', field: '' }
        } else {
            leftConcat.value = { kind: 'const_string', value: '' }
        }
        syncLeftConcat()
    }

    function updateLeftConcatField(field: string) {
        leftConcat.value = { kind: 'field', field }
        syncLeftConcat()
    }

    function updateLeftConcatValue(value: string) {
        leftConcat.value = { kind: 'const_string', value }
        syncLeftConcat()
    }

    function updateRightConcatKind(kind: 'field' | 'const_string') {
        if (kind === 'field') {
            rightConcat.value = { kind: 'field', field: '' }
        } else {
            rightConcat.value = { kind: 'const_string', value: '' }
        }
        syncRightConcat()
    }

    function updateRightConcatField(field: string) {
        rightConcat.value = { kind: 'field', field }
        syncRightConcat()
    }

    function updateRightConcatValue(value: string) {
        rightConcat.value = { kind: 'const_string', value }
        syncRightConcat()
    }

    function syncLeftConcat() {
        if (transformType.value === 'concat' && props.modelValue.type === 'concat') {
            const updated: Transform = {
                ...props.modelValue,
                left: { ...leftConcat.value },
            }
            emit('update:modelValue', updated)
        }
    }

    function syncRightConcat() {
        if (transformType.value === 'concat' && props.modelValue.type === 'concat') {
            const updated: Transform = {
                ...props.modelValue,
                right: { ...rightConcat.value },
            }
            emit('update:modelValue', updated)
        }
    }
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
</style>
