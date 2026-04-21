<template>
    <div class="dsl-config">
        <v-alert
            type="info"
            variant="tonal"
            density="comfortable"
            class="mb-4"
        >
            {{ t('workflows.dsl.guidance.pipeline_intro') }}
        </v-alert>

        <div
            v-if="stepsLocal.length === 0"
            class="mb-4"
        >
            <div class="text-subtitle-2 mb-2">
                {{ t('workflows.dsl.templates.title') }}
            </div>
            <div class="template-grid">
                <v-card
                    v-for="template in workflowTemplates"
                    :key="template.id"
                    variant="outlined"
                    class="template-card"
                    @click="applyTemplate(template.id)"
                >
                    <v-card-title class="text-body-1">
                        {{ t(`workflows.dsl.templates.items.${template.id}.title`) }}
                    </v-card-title>
                    <v-card-text class="text-body-2 text-medium-emphasis">
                        {{ t(`workflows.dsl.templates.items.${template.id}.description`) }}
                    </v-card-text>
                </v-card>
            </div>
        </div>

        <div class="d-flex align-center justify-space-between mb-2">
            <div class="text-subtitle-2">{{ t('workflows.dsl.steps_title') }}</div>
            <v-btn
                size="small"
                variant="outlined"
                color="primary"
                @click="addStep"
                >{{ t('workflows.dsl.add_step') }}
            </v-btn>
        </div>
        <v-alert
            v-if="loadError"
            type="error"
            density="compact"
            class="mb-2"
            >{{ loadError }}
        </v-alert>
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
                    <div class="step-title-wrap">
                        <div class="text-subtitle-2">
                            {{ t('workflows.dsl.step_label', { n: String(idx + 1) }) }}
                        </div>
                        <div class="text-body-2 text-medium-emphasis">
                            {{ buildStepSummary(step, t) }}
                        </div>
                        <div class="step-stats">
                            <v-chip
                                v-for="stat in getStepStats(step, t)"
                                :key="`${idx}-${stat.label}`"
                                size="small"
                                variant="tonal"
                            >
                                {{ stat.label }}: {{ stat.value }}
                            </v-chip>
                        </div>
                    </div>
                    <template #actions>
                        <div class="d-flex align-center ga-1">
                            <v-btn
                                size="x-small"
                                variant="text"
                                color="primary"
                                @click.stop="duplicateStep(idx)"
                            >
                                <SmartIcon
                                    icon="copy"
                                    :size="16"
                                />
                            </v-btn>
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
                        </div>
                    </template>
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

        <PostRunActionsEditor
            v-if="capabilitiesStore.workflowMailConfigured"
            :model-value="onCompleteLocal"
            @update:model-value="updateOnComplete"
        />
    </div>
</template>

<script setup lang="ts">
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { onMounted, ref, watch, nextTick, shallowRef, computed } from 'vue'
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
    import { createWorkflowTemplates } from './dsl/templates'
    import { buildStepSummary, getStepStats } from './dsl/summary'
    import { useCapabilitiesStore } from '@/stores/capabilities'
    import DslStepEditor from './dsl/DslStepEditor.vue'
    import PostRunActionsEditor from './dsl/PostRunActionsEditor.vue'
    import type { OnComplete } from '@/types/schemas/dsl'

    const props = defineProps<{
        modelValue: DslStep[]
        workflowUuid?: string | null
        onComplete?: OnComplete | null
    }>()
    const emit = defineEmits<{
        (e: 'update:modelValue', value: DslStep[]): void
        (e: 'update:onComplete', value: OnComplete | null): void
    }>()

    const loading = ref(false)
    const loadError = ref<string | null>(null)
    // Use shallowRef to avoid deep reactivity issues
    const stepsLocal = shallowRef<DslStep[]>([])
    const onCompleteLocal = ref<OnComplete | null>(props.onComplete ?? null)
    const openPanels = ref<number[]>([])
    const { t } = useTranslations()
    const capabilitiesStore = useCapabilitiesStore()
    const allTemplates = createWorkflowTemplates()
    const workflowTemplates = computed(() =>
        allTemplates.filter(
            tpl => tpl.id !== 'email_notification' || capabilitiesStore.workflowMailConfigured
        )
    )

    const { loadEntityDefinitions } = useEntityDefinitions()

    // Track if we're currently updating to prevent recursive loops
    let isUpdating = false

    // Helper functions for context passing
    function getPreviousStepFields(stepIndex: number): string[] {
        if (stepIndex === 0) {
            return []
        }
        const previousStep = stepsLocal.value[stepIndex - 1]
        const { normalizedFields } = useNormalizedFields(previousStep)
        return normalizedFields.value
    }

    function getNormalizedFields(stepIndex: number): string[] {
        const step = stepsLocal.value[stepIndex]
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

    function applyTemplate(templateId: (typeof allTemplates)[number]['id']) {
        const template = allTemplates.find(item => item.id === templateId)
        if (!template) {
            return
        }
        const steps = sanitizeDslSteps(template.steps)
        steps.forEach(step => {
            ensureCsvOptions(step)
            ensureEntityFilter(step)
        })
        stepsLocal.value = steps
        openPanels.value = [0]
        void nextTick(() => {
            emit('update:modelValue', [...stepsLocal.value])
        })
    }

    function duplicateStep(idx: number) {
        const original = stepsLocal.value[idx] as DslStep | undefined
        if (!original) {
            return
        }
        const duplicated = sanitizeDslSteps([structuredClone(original)])[0]
        ensureCsvOptions(duplicated)
        ensureEntityFilter(duplicated)
        const updated = [...stepsLocal.value]
        updated.splice(idx + 1, 0, duplicated)
        stepsLocal.value = updated
        openPanels.value = [idx + 1]
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

    function updateOnComplete(value: OnComplete | null) {
        onCompleteLocal.value = value
        emit('update:onComplete', value)
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

    // Sync onCompleteLocal when the onComplete prop changes from the parent
    watch(
        () => props.onComplete,
        newValue => {
            const incoming = newValue ?? null
            const current = JSON.stringify(onCompleteLocal.value)
            const next = JSON.stringify(incoming)
            if (current !== next) {
                onCompleteLocal.value = incoming
            }
        }
    )

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
            const newStr = JSON.stringify(newValue)
            if (currentStr !== newStr) {
                // Preserve currently open panels
                const preservedOpenPanels = [...openPanels.value]
                isUpdating = true
                try {
                    // Sanitize steps when loading from props
                    const sanitized = sanitizeDslSteps(newValue)
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

    .template-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 12px;
    }

    .template-card {
        cursor: pointer;
        transition:
            border-color 0.18s ease,
            transform 0.18s ease,
            box-shadow 0.18s ease;
    }

    .template-card:hover {
        border-color: rgba(var(--v-theme-primary), 0.55);
        transform: translateY(-2px);
        box-shadow: 0 10px 24px rgba(15, 23, 42, 0.08);
    }

    .step-title-wrap {
        display: flex;
        flex-direction: column;
        gap: 6px;
        min-width: 0;
    }

    .step-stats {
        display: flex;
        gap: 8px;
        flex-wrap: wrap;
    }
</style>
