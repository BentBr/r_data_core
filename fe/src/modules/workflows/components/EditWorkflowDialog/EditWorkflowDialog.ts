import { computed, ref, watch, defineComponent, PropType } from 'vue'
import JsonEditorVue from 'json-editor-vue'
import { typedHttpClient } from '@/api/typed-client'
import { ValidationError } from '@/api/typed-client'
import { getDialogMaxWidth } from '@/design-system/components'
import DslConfigurator from '../DslConfigurator/index.vue'
import WorkflowFormFields from '../WorkflowFormFields/index.vue'
import VersionHistory from '@/shared/components/VersionHistory/index.vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { computeDiffRows } from '@/utils/versionDiff'
import type { DslStep } from '../dsl/dsl-utils'
import { sanitizeDslSteps, ensureCsvOptions, ensureEntityFilter } from '../dsl/dsl-utils'
import type { WorkflowConfig } from '@/types/schemas/workflow'

export default defineComponent({
    name: 'EditWorkflowDialog',
    components: {
        DslConfigurator,
        WorkflowFormFields,
        VersionHistory,
        JsonEditorVue,
    },
    props: {
        modelValue: {
            type: Boolean,
            required: true,
        },
        workflowUuid: {
            type: String as PropType<string | null>,
            default: null,
        },
    },
    emits: ['update:modelValue', 'updated'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const model = computed({
            get: () => props.modelValue,
            set: v => emit('update:modelValue', v),
        })
        const loading = ref(false)
        const formRef = ref()
        const activeTab = ref('edit')

        // Versions/diff
        const versions = ref<
            Array<{
                version_number: number
                created_at: string
                created_by?: string | null
                created_by_name?: string | null
            }>
        >([])
        const versionHistoryRef = ref<InstanceType<typeof VersionHistory>>()

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

        // Bidirectional sync between steps and JSON flags
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

        // Check if any step has from.api source type (accepts POST, no cron needed)
        const hasApiSource = computed(() => {
            return steps.value.some((step: DslStep) => {
                if (step.from.type === 'format' && step.from.source.source_type === 'api') {
                    // from.api without endpoint field = accepts POST
                    return !step.from.source.config.endpoint
                }
                return false
            })
        })

        // Check if any step has to.format.output.mode === 'api' (exports via GET, no cron needed)
        const hasApiOutput = computed(() => {
            return steps.value.some((step: DslStep) => {
                if (step.to.type === 'format') {
                    const output = step.to.output
                    if (typeof output === 'string') {
                        return output === 'api'
                    }
                    if (typeof output === 'object' && 'mode' in output) {
                        return output.mode === 'api'
                    }
                }
                return false
            })
        })

        const loadVersions = async () => {
            if (!props.workflowUuid) {
                return
            }
            try {
                versions.value = await typedHttpClient.listWorkflowVersions(props.workflowUuid)
            } catch (e) {
                console.error('Failed to load versions:', e)
            }
        }

        watch(
            () => props.modelValue,
            open => {
                if (open) {
                    void loadDetails()
                    void loadVersions()
                }
            }
        )

        watch(
            () => props.workflowUuid,
            async () => {
                if (props.workflowUuid) {
                    await loadVersions()
                }
            },
            { immediate: true }
        )

        const handleVersionCompare = async (versionA: number, versionB: number) => {
            if (!props.workflowUuid) {
                return
            }
            try {
                const [a, b] = await Promise.all([
                    typedHttpClient.getWorkflowVersion(props.workflowUuid, versionA),
                    typedHttpClient.getWorkflowVersion(props.workflowUuid, versionB),
                ])
                const aData = a.data as Record<string, unknown>
                const bData = b.data as Record<string, unknown>
                const diffRows = computeDiffRows(aData, bData)
                versionHistoryRef.value?.updateDiffRows(diffRows)
            } catch (e) {
                console.error('Failed to load diff:', e)
            }
        }

        async function loadDetails() {
            if (!props.workflowUuid) {
                return
            }
            loading.value = true
            try {
                const data = await typedHttpClient.getWorkflow(props.workflowUuid)
                form.value.name = data.name
                form.value.description = data.description ?? ''
                form.value.kind = data.kind as 'consumer' | 'provider'
                form.value.enabled = data.enabled
                form.value.schedule_cron = data.schedule_cron ?? ''
                form.value.versioning_disabled =
                    'versioning_disabled' in data && typeof data.versioning_disabled === 'boolean'
                        ? data.versioning_disabled
                        : false
                configJson.value = JSON.stringify(data.config, null, 2)
                try {
                    const cfg = data.config as { steps?: DslStep[] }
                    isSyncingSteps = true
                    steps.value = Array.isArray(cfg.steps) ? cfg.steps : []
                    // Reset flag after next tick
                    setTimeout(() => {
                        isSyncingSteps = false
                    }, 0)
                } catch {
                    steps.value = []
                    isSyncingSteps = false
                }
            } finally {
                loading.value = false
            }
        }

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

        function cancel() {
            model.value = false
        }

        function parseJson(input: string): unknown | undefined {
            if (!input.trim()) {
                return undefined
            }
            try {
                return JSON.parse(input)
            } catch {
                return null
            }
        }

        async function submit() {
            if (!props.workflowUuid) {
                return
            }
            configError.value = null
            cronError.value = null
            const parsedConfig = parseJson(configJson.value)
            if (parsedConfig === null) {
                configError.value = 'Invalid JSON'
                return
            }
            try {
                const config = parsedConfig as { steps?: unknown[] }
                const steps = Array.isArray(config.steps) ? config.steps : null
                if (!steps || steps.length === 0) {
                    configError.value = 'DSL steps are required'
                    return
                }
                await typedHttpClient.validateDsl(steps as import('@/types/schemas').DslStep[])
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
                        configError.value = e.message || 'Invalid DSL'
                    }
                    return
                }
                if (e && typeof e === 'object' && 'violations' in e) {
                    const violations = (e as { violations?: Array<{ message?: string }> }).violations
                    if (violations && violations.length > 0) {
                        const v = violations[0]
                        configError.value = v.message ?? 'Invalid DSL'
                        return
                    }
                }
                configError.value = e instanceof Error ? e.message : 'Invalid DSL'
                return
            }

            loading.value = true
            try {
                await typedHttpClient.updateWorkflow(props.workflowUuid, {
                    name: form.value.name,
                    description: form.value.description || null,
                    kind: form.value.kind,
                    enabled: form.value.enabled,
                    schedule_cron:
                        hasApiSource.value || hasApiOutput.value
                            ? null
                            : (form.value.schedule_cron ?? null),
                    config: parsedConfig as WorkflowConfig,
                    versioning_disabled: form.value.versioning_disabled,
                })
                emit('updated')
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

        // Sync config JSON when steps change
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
            { deep: false }
        )

        // Sync steps when JSON changes manually
        watch(
            () => configJson.value,
            jsonStr => {
                if (isSyncingSteps || isSyncingJson) {
                    return
                }
                try {
                    const parsed = parseJson(jsonStr)
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
            activeTab,
            versions,
            versionHistoryRef,
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
            handleVersionCompare,
            onCronChange,
            cancel,
            submit,
            getDialogMaxWidth,
            configObject,
        }
    },
})
