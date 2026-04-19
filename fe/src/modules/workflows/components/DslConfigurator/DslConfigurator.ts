import { onMounted, ref, watch, nextTick, shallowRef, defineComponent, PropType } from 'vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useEntityDefinitions } from '@/shared/composables/useEntityDefinitions'
import { useNormalizedFields } from '../dsl/useNormalizedFields'
import type { DslStep } from '../dsl/dsl-utils'
import {
    sanitizeDslSteps,
    defaultStep,
    ensureCsvOptions,
    ensureEntityFilter,
} from '../dsl/dsl-utils'
import { createWorkflowTemplates } from '../dsl/templates'
import { buildStepSummary, getStepStats } from '../dsl/summary'
import DslStepEditor from '../dsl/DslStepEditor/index.vue'

export default defineComponent({
    name: 'DslConfigurator',
    components: {
        SmartIcon,
        DslStepEditor,
    },
    props: {
        modelValue: {
            type: Array as PropType<DslStep[]>,
            required: true,
        },
        workflowUuid: {
            type: String as PropType<string | null>,
            default: null,
        },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const loading = ref(false)
        const loadError = ref<string | null>(null)
        // Use shallowRef to avoid deep reactivity issues
        const stepsLocal = shallowRef<DslStep[]>([])
        const openPanels = ref<number[]>([])
        const { t } = useTranslations()
        const workflowTemplates = createWorkflowTemplates()

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

        function applyTemplate(templateId: (typeof workflowTemplates)[number]['id']) {
            const template = workflowTemplates.find((item: (typeof workflowTemplates)[number]) => item.id === templateId)
            if (!template) {
                return
            }
            const steps = sanitizeDslSteps(template.steps)
            steps.forEach((step: DslStep) => {
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
        watch(
            () => props.modelValue,
            newValue => {
                if (isUpdating) {
                    return
                }
                const currentStr = JSON.stringify(stepsLocal.value)
                const newStr = JSON.stringify(newValue)
                if (currentStr !== newStr) {
                    const preservedOpenPanels = [...openPanels.value]
                    isUpdating = true
                    try {
                        const sanitized = sanitizeDslSteps(newValue)
                        sanitized.forEach((s: DslStep) => {
                            ensureCsvOptions(s)
                            ensureEntityFilter(s)
                        })
                        stepsLocal.value = sanitized
                        void nextTick(() => {
                            if (
                                preservedOpenPanels.length > 0 &&
                                preservedOpenPanels.every(idx => idx < sanitized.length)
                            ) {
                                openPanels.value = preservedOpenPanels
                            }
                        })
                    } finally {
                        void nextTick(() => {
                            isUpdating = false
                        })
                    }
                }
            },
            { immediate: true, deep: false }
        )

        return {
            t,
            loading,
            loadError,
            stepsLocal,
            openPanels,
            workflowTemplates,
            getPreviousStepFields,
            getNormalizedFields,
            updateStep,
            addStep,
            applyTemplate,
            duplicateStep,
            removeStep,
            buildStepSummary,
            getStepStats,
        }
    },
})
