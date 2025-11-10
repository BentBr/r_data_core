<template>
    <div>
        <DslFromEditor
            :model-value="modelValue.from"
            @update:model-value="updateFrom"
        />
        <div class="mb-4" />
        <DslTransformEditor
            :model-value="modelValue.transform"
            @update:model-value="updateTransform"
        />
        <div class="mb-4" />
        <DslToEditor
            :model-value="modelValue.to"
            @update:model-value="updateTo"
        />
    </div>
</template>

<script setup lang="ts">
import type { DslStep } from './dsl-utils'
import DslFromEditor from './DslFromEditor.vue'
import DslTransformEditor from './DslTransformEditor.vue'
import DslToEditor from './DslToEditor.vue'

const props = defineProps<{
    modelValue: DslStep
}>()

const emit = defineEmits<{ (e: 'update:modelValue', value: DslStep): void }>()

function updateFrom(from: any) {
    emit('update:modelValue', { ...props.modelValue, from })
}

function updateTransform(transform: any) {
    emit('update:modelValue', { ...props.modelValue, transform })
}

function updateTo(to: any) {
    emit('update:modelValue', { ...props.modelValue, to })
}
</script>

