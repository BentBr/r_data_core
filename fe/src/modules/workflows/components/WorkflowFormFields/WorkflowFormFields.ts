import { computed, watch, defineComponent, PropType } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { DslStep } from '../dsl/dsl-utils'

type WorkflowForm = {
    name: string
    description: string
    kind: 'consumer' | 'provider'
    enabled: boolean
    schedule_cron: string | null
    versioning_disabled: boolean
}

export default defineComponent({
    name: 'WorkflowFormFields',
    props: {
        form: {
            type: Object as PropType<WorkflowForm>,
            required: true,
        },
        steps: {
            type: Array as PropType<DslStep[]>,
            required: true,
        },
        cronError: {
            type: String as PropType<string | null>,
            default: null,
        },
        cronHelp: {
            type: String,
            required: true,
        },
        nextRuns: {
            type: Array as PropType<string[]>,
            required: true,
        },
    },
    emits: ['update:form', 'update:cronError', 'update:nextRuns', 'cronChange'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const kinds = [
            { label: 'Consumer', value: 'consumer' },
            { label: 'Provider', value: 'provider' },
        ]

        const rules = {
            required: (v: unknown) => (!!v && String(v).trim().length > 0) || t('validation.required'),
        }

        // Check if any step has from.api source type (accepts POST, no cron needed)
        const hasApiSource = computed(() => {
            return props.steps.some((step: DslStep) => {
                if (step.from.type === 'format' && step.from.source.source_type === 'api') {
                    // from.api without endpoint field = accepts POST
                    return !step.from.source.config.endpoint
                }
                return false
            })
        })

        // Check if any step has to.format.output.mode === 'api' (exports via GET, no cron needed)
        const hasApiOutput = computed(() => {
            if (props.steps.length === 0) {
                return false
            }
            return props.steps.some(step => {
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

        // Watch hasApiSource and hasApiOutput and clear cron when API source or output is used
        watch([hasApiSource, hasApiOutput], ([isApiSource, isApiOutput]) => {
            if (isApiSource || isApiOutput) {
                emit('update:cronError', null)
                emit('update:form', { ...props.form, schedule_cron: null })
                emit('update:nextRuns', [])
            }
        })

        let cronDebounce: ReturnType<typeof setTimeout> | null = null

        async function onCronChange(value: string) {
            emit('cronChange', value)
            if (hasApiSource.value || hasApiOutput.value) {
                emit('update:cronError', null)
                emit('update:nextRuns', [])
                return
            }
            emit('update:cronError', null)
            if (cronDebounce) {
                clearTimeout(cronDebounce)
            }
            if (!value.trim()) {
                emit('update:nextRuns', [])
                return
            }
            cronDebounce = setTimeout(() => {
                void (async () => {
                    try {
                        const runs = await typedHttpClient.previewCron(value)
                        emit('update:nextRuns', runs)
                    } catch {
                        emit('update:nextRuns', [])
                    }
                })()
            }, 350)
        }

        return {
            t,
            kinds,
            rules,
            hasApiSource,
            hasApiOutput,
            onCronChange,
            emit,
        }
    },
})
