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
                :key="`step-${idx}`"
            >
                <v-expansion-panel-title>
                    {{ t('workflows.dsl.step_label', { n: String(idx + 1) }) }} — from:
                    {{ step.from?.type || '-' }}, transform: {{ step.transform?.type || '-' }}, to:
                    {{ step.to?.type || '-' }}
                    <v-spacer />
                    <v-btn
                        size="x-small"
                        variant="text"
                        color="error"
                        @click.stop="removeStep(idx)"
                    >
                        <SmartIcon
                            icon="trash-2"
                            :size="16"
                        />
                    </v-btn>
                </v-expansion-panel-title>
                <v-expansion-panel-text>
                    <DslStepEditor
                        :model-value="step"
                        :workflow-uuid="workflowUuid"
                        :step-index="idx"
                        :previous-step-fields="getPreviousStepFields(idx)"
                        :normalized-fields="getNormalizedFields(idx)"
                        :is-last-step="idx === stepsLocal.length - 1"
                        @update:model-value="updateStep(idx, $event)"
                    />
                </v-expansion-panel-text>
            </v-expansion-panel>
        </v-expansion-panels>
    </div>
</template>

<script setup lang="ts">
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { onMounted, ref, watch, nextTick, shallowRef } from 'vue'
    import { typedHttpClient } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import { useEntityDefinitions } from '@/composables/useEntityDefinitions'
    import { useNormalizedFields } from './dsl/useNormalizedFields'
    import type { DslStep } from './dsl/dsl-utils'
    import {
        sanitizeDslSteps,
        defaultStep,
        ensureCsvOptions,
        ensureEntityFilter,
    } from './dsl/dsl-utils'
    import DslStepEditor from './dsl/DslStepEditor.vue'

    const props = defineProps<{
        modelValue: DslStep[]
        workflowUuid?: string | null
    }>()
    const emit = defineEmits<{ (e: 'update:modelValue', value: DslStep[]): void }>()

    const loading = ref(false)
    const loadError = ref<string | null>(null)
    // Use shallowRef to avoid deep reactivity issues
    const stepsLocal = shallowRef<DslStep[]>([])
    const openPanels = ref<number[]>([])
    const { t } = useTranslations()

    const { loadEntityDefinitions } = useEntityDefinitions()

    // Track if we're currently updating to prevent recursive loops
    let isUpdating = false

    // Helper functions for context passing
    function getPreviousStepFields(stepIndex: number): string[] {
        if (stepIndex === 0) {
            return []
        }
        const previousStep = stepsLocal.value[stepIndex - 1]
        if (!previousStep) {
            return []
        }
        const { normalizedFields } = useNormalizedFields(previousStep)
        return normalizedFields.value
    }

    function getNormalizedFields(stepIndex: number): string[] {
        const step = stepsLocal.value[stepIndex]
        if (!step) {
            return []
        }
        const { normalizedFields } = useNormalizedFields(step)
        return normalizedFields.value
    }

    function updateStep(idx: number, newStep: DslStep) {
        if (isUpdating) {
            return
        }
        const updated = [...stepsLocal.value]
        // Sanitize the step before storing
        const sanitized = sanitizeDslSteps([newStep])[0]
        ensureCsvOptions(sanitized)
        ensureEntityFilter(sanitized)
        updated[idx] = sanitized
        stepsLocal.value = updated
        // Emit change after next tick to batch updates
        void nextTick(() => {
            if (!isUpdating) {
                emit('update:modelValue', [...stepsLocal.value])
            }
        })
    }

    function addStep() {
        const newStep = defaultStep()
        stepsLocal.value = [...stepsLocal.value, newStep]
        openPanels.value = [stepsLocal.value.length - 1]
        void nextTick(() => {
            emit('update:modelValue', [...stepsLocal.value])
        })
    }

    function removeStep(idx: number) {
        const updated = [...stepsLocal.value]
        updated.splice(idx, 1)
        stepsLocal.value = updated
        void nextTick(() => {
            emit('update:modelValue', [...stepsLocal.value])
        })
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
        } catch (e: unknown) {
            loadError.value = e instanceof Error ? e.message : 'Failed to load DSL options'
        } finally {
            loading.value = false
        }
    })

    // Watch for prop changes and update local state
    // Preserve openPanels state when props change to prevent auto-closing
    watch(
        () => props.modelValue,
        newValue => {
            if (isUpdating) {
                return
            }
            // Only update if actually different to prevent loops
            const currentStr = JSON.stringify(stepsLocal.value)
            const newStr = JSON.stringify(newValue || [])
            if (currentStr !== newStr) {
                // Preserve currently open panels
                const preservedOpenPanels = [...openPanels.value]
                isUpdating = true
                try {
                    // Sanitize steps when loading from props
                    const sanitized = sanitizeDslSteps(newValue || [])
                    sanitized.forEach(s => {
                        ensureCsvOptions(s)
                        ensureEntityFilter(s)
                    })
                    stepsLocal.value = sanitized
                    // Restore open panels after update, but only if the number of steps hasn't changed
                    void nextTick(() => {
                        if (
                            preservedOpenPanels.length > 0 &&
                            preservedOpenPanels.every(idx => idx < sanitized.length)
                        ) {
                            openPanels.value = preservedOpenPanels
                        }
                    })
                } finally {
                    // Reset flag after next tick
                    void nextTick(() => {
                        isUpdating = false
                    })
                }
            }
        },
        { immediate: true, deep: false } // Use shallow watch to avoid deep reactivity
    )
</script>

<style scoped>
    .dsl-config {
        width: 100%;
    }
</style>
