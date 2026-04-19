import { computed, ref, watch, defineComponent } from 'vue'
import JsonEditorVue from 'json-editor-vue'
import { typedHttpClient, ValidationError } from '@/api/typed-client'
import { useTranslations } from '@/shared/composables/useTranslations'
import { getDialogMaxWidth } from '@/design-system/components'
import DslConfigurator from '../DslConfigurator/index.vue'
import WorkflowFormFields from '../WorkflowFormFields/index.vue'
import type { DslStep } from '../dsl/dsl-utils'
import { sanitizeDslSteps, ensureCsvOptions, ensureEntityFilter } from '../dsl/dsl-utils'

export default defineComponent({
    name: 'CreateWorkflowDialog',
    components: {
        DslConfigurator,
        WorkflowFormFields,
        JsonEditorVue,
    },
    props: {
        modelValue: {
            type: Boolean,
            required: true,
        },
    },
    emits: ['update:modelValue', 'created'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const model = computed({
            get: () => props.modelValue,
            set: v => emit('update:modelValue', v),
        })
        const loading = ref(false)
        const formRef = ref()

        const form = ref({
            name: '',
            description: '',
            kind: 'consumer' as 'consumer' | 'provider',
            enabled: true,
            schedule_cron: '' as string | null,
            versioning_disabled: false,
        })

        const configJson = ref('')
        const configError = ref<string | null>(null)
        const steps = ref<DslStep[]>([])
        const cronError = ref<string | null>(null)
        const cronHelp = ref<string>(
            'Use standard 5-field cron (min hour day month dow), e.g. "*/5 * * * *"'
        )
        const nextRuns = ref<string[]>([])
        const expansionPanels = ref<number[]>([])
        let cronDebounce: ReturnType<typeof setTimeout> | null = null

        // Bidirectional sync flags
        let isSyncingSteps = false
        let isSyncingJson = false

        // Computed for JSON editor to handle object instead of string
        const configObject = computed({
            get: () => {
                try {
                    return JSON.parse(configJson.value || '{}')
                } catch {
                    return { steps: steps.value }
                }
            },
            set: (val) => {
                if (isSyncingJson || isSyncingSteps) return
                try {
                    const newStr = JSON.stringify(val, null, 2)
                    if (configJson.value !== newStr) {
                        isSyncingJson = true
                        configJson.value = newStr
                        setTimeout(() => { isSyncingJson = false }, 0)
                    }
                } catch {
                    // ignore
                }
            }
        })

        const hasApiOutput = computed(() => {
            if (steps.value.length === 0) {
                return false
            }
            return steps.value.some(step => {
                if (step.to.type === 'format' && step.to.output.mode === 'api') {
                    return true
                }
                return false
            })
        })

        const hasApiSource = computed(() => {
            return steps.value.some((step: DslStep) => {
                if (step.from.type === 'format' && step.from.source.source_type === 'api') {
                    return !step.from.source.config.endpoint
                }
                return false
            })
        })

        watch([hasApiSource, hasApiOutput], ([isApiSource, isApiOutput]) => {
            if (isApiSource || isApiOutput) {
                cronError.value = null
                form.value.schedule_cron = null
                nextRuns.value = []
                if (cronDebounce) {
                    clearTimeout(cronDebounce)
                    cronDebounce = null
                }
            }
        })

        async function onCronChange(value: string) {
            if (hasApiSource.value || hasApiOutput.value) {
                cronError.value = null
                nextRuns.value = []
                return
            }
            cronError.value = null
            if (cronDebounce) {
                clearTimeout(cronDebounce)
            }
            if (!value.trim()) {
                nextRuns.value = []
                return
            }
            cronDebounce = setTimeout(() => {
                void (async () => {
                    try {
                        nextRuns.value = await typedHttpClient.previewCron(value)
                    } catch {
                        nextRuns.value = []
                    }
                })()
            }, 350)
        }

        function parseJson(input: string): { parsed: unknown; error: string | null } {
            if (!input.trim()) {
                return { parsed: undefined, error: null }
            }
            try {
                return { parsed: JSON.parse(input), error: null }
            } catch (e) {
                const errorMessage = e instanceof Error ? e.message : String(e)
                return { parsed: null, error: errorMessage }
            }
        }

        function cancel() {
            model.value = false
        }

        async function submit() {
            configError.value = null
            cronError.value = null
            const { parsed: parsedConfig, error: parseError } = parseJson(configJson.value)
            if (parseError) {
                configError.value = `${t('workflows.create.json_invalid')}: ${parseError}`
                return
            }
            if (parsedConfig === null || parsedConfig === undefined) {
                configError.value = t('workflows.create.json_invalid')
                return
            }
            try {
                const configObj = parsedConfig as Record<string, unknown> | null
                const configSteps = Array.isArray(configObj?.steps) ? configObj.steps : null
                if (!configSteps || configSteps.length === 0) {
                    configError.value = t('workflows.create.dsl_required')
                    return
                }
                await typedHttpClient.validateDsl(configSteps as DslStep[])
            } catch (e: unknown) {
                if (e instanceof ValidationError) {
                    const violations = e.violations
                    if (violations.length > 0) {
                        const errorMessages = violations.map(v => {
                            const fieldName = v.field && v.field !== 'dsl' ? `${v.field}: ` : ''
                            return `${fieldName}${v.message}`
                        })
                        configError.value = errorMessages.join('; ')
                    } else {
                        configError.value = e.message || t('workflows.create.dsl_invalid')
                    }
                    return
                }
                const errorObj = e as Record<string, unknown> | null
                if (errorObj?.violations) {
                    const violations = errorObj.violations as Array<{ message?: string }>
                    const v = violations[0]
                    configError.value = v.message ?? t('workflows.create.dsl_invalid')
                    return
                }
                configError.value = e instanceof Error ? e.message : t('workflows.create.dsl_invalid')
                return
            }

            loading.value = true
            try {
                const payload = {
                    name: form.value.name,
                    description: form.value.description,
                    kind: form.value.kind,
                    enabled: form.value.enabled,
                    schedule_cron:
                        hasApiSource.value || hasApiOutput.value ? null : form.value.schedule_cron,
                    config: parsedConfig as import('@/types/schemas').WorkflowConfig,
                    versioning_disabled: form.value.versioning_disabled,
                }
                const res = await typedHttpClient.createWorkflow(payload)
                emit('created', res.uuid)
                model.value = false
            } catch (e: unknown) {
                if (e instanceof ValidationError) {
                    const cronViolation = e.violations.find(v => v.field === 'schedule_cron')
                    if (cronViolation) {
                        cronError.value = cronViolation.message
                    }
                    return
                }
                throw e
            } finally {
                loading.value = false
            }
        }

        watch(
            () => steps.value,
            v => {
                if (isSyncingSteps || isSyncingJson) {
                    return
                }
                try {
                    const newJson = JSON.stringify({ steps: v }, null, 2)
                    if (configJson.value !== newJson) {
                        isSyncingJson = true
                        configJson.value = newJson
                        setTimeout(() => {
                            isSyncingJson = false
                        }, 0)
                    }
                } catch {
                    // ignore
                }
            },
            { deep: true }
        )

        watch(
            () => configJson.value,
            jsonStr => {
                if (isSyncingSteps || isSyncingJson) {
                    return
                }
                try {
                    const { parsed } = parseJson(jsonStr)
                    if (parsed && typeof parsed === 'object') {
                        const config = parsed as { steps?: unknown[] }
                        if (Array.isArray(config.steps)) {
                            isSyncingSteps = true
                            const sanitized = sanitizeDslSteps(config.steps)
                            sanitized.forEach((s: DslStep) => {
                                ensureCsvOptions(s)
                                ensureEntityFilter(s)
                            })
                            steps.value = sanitized
                            setTimeout(() => {
                                isSyncingSteps = false
                            }, 0)
                        }
                    }
                } catch {
                    // ignore
                }
            }
        )

        return {
            t,
            model,
            loading,
            formRef,
            form,
            configJson,
            configError,
            steps,
            hasApiSource,
            hasApiOutput,
            cronError,
            cronHelp,
            nextRuns,
            expansionPanels,
            onCronChange,
            cancel,
            submit,
            getDialogMaxWidth,
            configObject,
        }
    },
})
