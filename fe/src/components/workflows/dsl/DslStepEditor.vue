<template>
    <div>
        <DslFromEditor
            :model-value="modelValue.from"
            :workflow-uuid="workflowUuid"
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
            :workflow-uuid="workflowUuid"
            @update:model-value="updateTo"
        />
    </div>
</template>

<script setup lang="ts">
    import type { DslStep, FromDef, Transform, ToDef } from './dsl-utils'
    import DslFromEditor from './DslFromEditor.vue'
    import DslTransformEditor from './DslTransformEditor.vue'
    import DslToEditor from './DslToEditor.vue'

    const props = defineProps<{
        modelValue: DslStep
        workflowUuid?: string | null
    }>()

    const emit = defineEmits<{ (e: 'update:modelValue', value: DslStep): void }>()

    function updateFrom(from: FromDef) {
        emit('update:modelValue', { ...props.modelValue, from })
    }

    function updateTransform(transform: Transform) {
        emit('update:modelValue', { ...props.modelValue, transform })
    }

    function updateTo(to: ToDef) {
        emit('update:modelValue', { ...props.modelValue, to })
    }
</script>
