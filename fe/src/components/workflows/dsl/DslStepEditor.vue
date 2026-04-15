<template>
    <div class="dsl-step-editor">
        <v-card
            variant="outlined"
            class="mb-4 pa-4"
        >
            <DslFromEditor
                :model-value="modelValue.from"
                :workflow-uuid="workflowUuid"
                :step-index="stepIndex"
                :previous-step-fields="previousStepFields"
                @update:model-value="updateFrom"
            />
        </v-card>
        <v-card
            variant="outlined"
            class="mb-4 pa-4"
        >
            <DslTransformEditor
                :model-value="modelValue.transform"
                :available-fields="normalizedFields"
                @update:model-value="updateTransform"
            />
        </v-card>
        <v-card
            variant="outlined"
            class="pa-4"
        >
            <DslToEditor
                :model-value="modelValue.to"
                :workflow-uuid="workflowUuid"
                :is-last-step="isLastStep"
                @update:model-value="updateTo"
            />
        </v-card>
    </div>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import type { DslStep, FromDef, Transform, ToDef } from './dsl-utils'
    import DslFromEditor from './DslFromEditor.vue'
    import DslTransformEditor from './DslTransformEditor.vue'
    import DslToEditor from './DslToEditor.vue'

    const props = defineProps<{
        modelValue: DslStep
        workflowUuid?: string | null
        stepIndex?: number
        previousStepFields?: string[]
        normalizedFields?: string[]
        isLastStep?: boolean
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

    // Default isLastStep
    const isLastStep = computed(() => props.isLastStep)
</script>
