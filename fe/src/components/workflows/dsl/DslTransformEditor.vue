<template>
    <div>
        <div class="text-caption mb-1">{{ t('workflows.dsl.transform') }}</div>
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
                    :model-value="(modelValue as any).target"
                    :label="t('workflows.dsl.target')"
                    density="comfortable"
                    @update:model-value="updateField('target', $event)"
                />
                <v-select
                    :model-value="(modelValue as any).op"
                    :items="ops"
                    :label="t('workflows.dsl.op')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 160px"
                    @update:model-value="updateField('op', $event)"
                />
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
                <v-text-field
                    v-if="left.kind === 'field'"
                    :model-value="left.field"
                    :label="t('workflows.dsl.left_field')"
                    density="comfortable"
                    @update:model-value="updateLeftField"
                />
                <v-text-field
                    v-else
                    :model-value="left.value"
                    :label="t('workflows.dsl.left_value')"
                    type="number"
                    density="comfortable"
                    @update:model-value="updateLeftValue"
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
                <v-text-field
                    v-if="right.kind === 'field'"
                    :model-value="right.field"
                    :label="t('workflows.dsl.right_field')"
                    density="comfortable"
                    @update:model-value="updateRightField"
                />
                <v-text-field
                    v-else
                    :model-value="right.value"
                    :label="t('workflows.dsl.right_value')"
                    type="number"
                    density="comfortable"
                    @update:model-value="updateRightValue"
                />
            </div>
        </template>
        <template v-else-if="transformType === 'concat'">
            <div class="d-flex ga-2">
                <v-text-field
                    :model-value="(modelValue as any).target"
                    :label="t('workflows.dsl.target')"
                    density="comfortable"
                    @update:model-value="updateField('target', $event)"
                />
                <v-text-field
                    :model-value="(modelValue as any).separator"
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
                <v-text-field
                    v-if="leftConcat.kind === 'field'"
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
                <v-text-field
                    v-if="rightConcat.kind === 'field'"
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
    </div>
</template>

<script setup lang="ts">
    import { ref, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import type { Transform, Operand, StringOperand } from './dsl-utils'

    const props = defineProps<{
        modelValue: Transform
    }>()

    const emit = defineEmits<{ (e: 'update:modelValue', value: Transform): void }>()

    const { t } = useTranslations()

    const ops = ['add', 'sub', 'mul', 'div']
    const operandKinds = ['field', 'const']
    const stringOperandKinds = ['field', 'const_string']
    const transformTypes = [
        { title: 'None', value: 'none' },
        { title: 'Arithmetic', value: 'arithmetic' },
        { title: 'Concat', value: 'concat' },
    ]

    const transformType = ref<'none' | 'arithmetic' | 'concat'>(props.modelValue.type || 'none')

    // Local operand editors
    const left = ref<Operand>({ kind: 'field', field: '' })
    const right = ref<Operand>({ kind: 'const', value: 0 })
    const leftConcat = ref<StringOperand>({ kind: 'field', field: '' })
    const rightConcat = ref<StringOperand>({ kind: 'const_string', value: '' })

    // Sync local operands from modelValue
    watch(
        () => props.modelValue,
        newTransform => {
            transformType.value = newTransform.type ?? 'none'
            if (newTransform.type === 'arithmetic') {
                left.value = { ...(newTransform as any).left } ?? { kind: 'field', field: '' }
                right.value = { ...(newTransform as any).right } ?? { kind: 'const', value: 0 }
            } else if (newTransform.type === 'concat') {
                leftConcat.value = { ...(newTransform as any).left } ?? { kind: 'field', field: '' }
                rightConcat.value = { ...(newTransform as any).right } ?? {
                    kind: 'const_string',
                    value: '',
                }
            }
        },
        { immediate: true, deep: true }
    )

    function updateField(field: string, value: any) {
        const updated: any = { ...props.modelValue }
        updated[field] = value
        emit('update:modelValue', updated as Transform)
    }

    function onTypeChange(newType: 'none' | 'arithmetic' | 'concat') {
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
        } else {
            newTransform = {
                type: 'concat',
                target: '',
                left: { kind: 'field', field: '' },
                separator: ' ',
                right: { kind: 'field', field: '' },
            }
            leftConcat.value = { kind: 'field', field: '' }
            rightConcat.value = { kind: 'field', field: '' }
        }
        emit('update:modelValue', newTransform)
    }

    // Arithmetic operand updates
    function updateLeftKind(kind: 'field' | 'const') {
        if (kind === 'field') {
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

    function updateRightKind(kind: 'field' | 'const') {
        if (kind === 'field') {
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
        if (transformType.value === 'arithmetic') {
            const updated: any = { ...props.modelValue }
            updated.left = { ...left.value }
            emit('update:modelValue', updated as Transform)
        }
    }

    function syncRight() {
        if (transformType.value === 'arithmetic') {
            const updated: any = { ...props.modelValue }
            updated.right = { ...right.value }
            emit('update:modelValue', updated as Transform)
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
        if (transformType.value === 'concat') {
            const updated: any = { ...props.modelValue }
            updated.left = { ...leftConcat.value }
            emit('update:modelValue', updated as Transform)
        }
    }

    function syncRightConcat() {
        if (transformType.value === 'concat') {
            const updated: any = { ...props.modelValue }
            updated.right = { ...rightConcat.value }
            emit('update:modelValue', updated as Transform)
        }
    }
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
</style>
