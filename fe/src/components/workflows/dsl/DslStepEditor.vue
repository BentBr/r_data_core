<template>
    <div class="step-editor">
        <v-alert
            variant="tonal"
            density="comfortable"
            class="mb-4"
        >
            <div class="text-subtitle-2 mb-1">
                {{ t('workflows.dsl.guidance.step_summary_title') }}
            </div>
            <div class="text-body-2">{{ stepSummary }}</div>
        </v-alert>

        <v-card
            variant="outlined"
            class="mb-4"
        >
            <v-card-title class="text-subtitle-2">{{
                t('workflows.dsl.guidance.sections.input_title')
            }}</v-card-title>
            <v-card-subtitle>
                {{ t('workflows.dsl.guidance.sections.input_description') }}
            </v-card-subtitle>
            <v-card-text>
                <DslFromEditor
                    :model-value="modelValue.from"
                    :workflow-uuid="workflowUuid"
                    :step-index="stepIndex"
                    :previous-step-fields="previousStepFields"
                    @update:model-value="updateFrom"
                />
            </v-card-text>
        </v-card>

        <v-card
            variant="outlined"
            class="mb-4"
        >
            <v-card-title class="text-subtitle-2">{{
                t('workflows.dsl.guidance.sections.transform_title')
            }}</v-card-title>
            <v-card-subtitle>
                {{ t('workflows.dsl.guidance.sections.transform_description') }}
            </v-card-subtitle>
            <v-card-text>
                <DslTransformEditor
                    :model-value="modelValue.transform"
                    :available-fields="normalizedFields"
                    @update:model-value="updateTransform"
                />
            </v-card-text>
        </v-card>

        <v-card variant="outlined">
            <v-card-title class="text-subtitle-2">{{
                t('workflows.dsl.guidance.sections.output_title')
            }}</v-card-title>
            <v-card-subtitle>
                {{ t('workflows.dsl.guidance.sections.output_description') }}
            </v-card-subtitle>
            <v-card-text>
                <DslToEditor
                    :model-value="modelValue.to"
                    :workflow-uuid="workflowUuid"
                    :is-last-step="isLastStep"
                    @update:model-value="updateTo"
                />
            </v-card-text>
        </v-card>
    </div>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import type { DslStep, FromDef, Transform, ToDef } from './contracts'
    import { buildStepSummary } from './summary'
    import { useTranslations } from '@/composables/useTranslations'
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
    const { t } = useTranslations()

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
    const stepSummary = computed(() => buildStepSummary(props.modelValue, t))
</script>

<style scoped>
    .step-editor :deep(.v-card-subtitle) {
        white-space: normal;
        line-height: 1.4;
    }
</style>
