<template>
    <v-dialog
        v-model="model"
        :max-width="getDialogMaxWidth('wide')"
    >
        <v-card>
            <v-card-title class="pa-6">{{ t('workflows.create.title') }}</v-card-title>
            <v-card-text class="pa-6">
                <v-form
                    ref="formRef"
                    @submit.prevent
                >
                    <WorkflowFormFields
                        :form="form"
                        :steps="steps"
                        :cron-error="cronError"
                        :cron-help="cronHelp"
                        :next-runs="nextRuns"
                        @update:form="form = $event"
                        @update:cron-error="cronError = $event"
                        @update:next-runs="nextRuns = $event"
                        @cron-change="onCronChange"
                    />

                    <div class="mt-4 mb-2">
                        <h3>{{ t('workflows.create.config_label') }}</h3>
                    </div>
                    <v-expansion-panels
                        v-model="expansionPanels"
                        class="mt-2"
                    >
                        <v-expansion-panel>
                            <v-expansion-panel-title
                                >{{ t('workflows.create.config_label') }}
                            </v-expansion-panel-title>
                            <v-expansion-panel-text>
                                <div class="mb-4">
                                    <DslConfigurator
                                        v-model="steps"
                                        :workflow-uuid="null"
                                    />
                                </div>
                                <v-textarea
                                    v-model="configJson"
                                    rows="8"
                                    auto-grow
                                    :error-messages="configError || ''"
                                />
                            </v-expansion-panel-text>
                        </v-expansion-panel>
                    </v-expansion-panels>
                </v-form>
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    variant="text"
                    color="mutedForeground"
                    @click="cancel"
                    >{{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="flat"
                    :loading="loading"
                    @click="submit"
                    >{{ t('workflows.create.create_button') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { computed, ref, watch } from 'vue'
    import { typedHttpClient, ValidationError } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import { getDialogMaxWidth } from '@/design-system/components'
    import DslConfigurator from './DslConfigurator.vue'
    import WorkflowFormFields from './WorkflowFormFields.vue'
    import type { DslStep } from './dsl/dsl-utils'
    import { sanitizeDslSteps, ensureCsvOptions, ensureEntityFilter } from './dsl/dsl-utils'

    const props = defineProps<{ modelValue: boolean }>()
    const emit = defineEmits<{
        (e: 'update:modelValue', value: boolean): void
        (e: 'created', uuid: string): void
    }>()

    const { t } = useTranslations()
    const model = computed({ get: () => props.modelValue, set: v => emit('update:modelValue', v) })
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

    // Check if any step has from.api source type (accepts POST, no cron needed)
    const hasApiOutput = computed(() => {
        if (!steps.value || steps.value.length === 0) {
            return false
        }
        return steps.value.some(step => {
            if (step.to?.type === 'format' && step.to.output?.mode === 'api') {
                return true
            }
            return false
        })
    })

    const hasApiSource = computed(() => {
        return steps.value.some((step: DslStep) => {
            if (step.from?.type === 'format' && step.from?.source?.source_type === 'api') {
                // from.api without endpoint field = accepts POST
                return !step.from?.source?.config?.endpoint
            }
            return false
        })
    })

    // Watch hasApiSource and hasApiOutput and clear cron when API source or output is used
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
        // Skip validation if API source is used
        if (hasApiSource.value || hasApiOutput.value) {
            cronError.value = null
            nextRuns.value = []
            return
        }
        cronError.value = null
        if (cronDebounce) {
            clearTimeout(cronDebounce)
        }
        if (!value?.trim()) {
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
        if (!input?.trim()) {
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
        // Use whatever is in configJson (could be manually edited or synced from steps)
        // No need to sync steps to JSON here - the watch handles that automatically
        const { parsed: parsedConfig, error: parseError } = parseJson(configJson.value)
        if (parseError) {
            configError.value = `${t('workflows.create.json_invalid')}: ${parseError}`
            return
        }
        if (parsedConfig === null || parsedConfig === undefined) {
            configError.value = t('workflows.create.json_invalid')
            return
        }
        // Strict DSL presence and validation against BE
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
                // Handle Symfony-style validation errors
                const violations = e.violations || []
                if (violations.length > 0) {
                    // Show all violations, with field names if available
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
                configError.value = v?.message ?? t('workflows.create.dsl_invalid')
                return
            }
            configError.value = e instanceof Error ? e.message : t('workflows.create.dsl_invalid')
            return
        }

        loading.value = true
        try {
            const payload = {
                name: form.value.name,
                description: form.value.description ?? null,
                kind: form.value.kind,
                enabled: form.value.enabled,
                // Set schedule_cron to null when API source or API output is used
                schedule_cron:
                    hasApiSource.value || hasApiOutput.value
                        ? null
                        : (form.value.schedule_cron ?? null),
                config: (parsedConfig ?? {}) as import('@/types/schemas').WorkflowConfig,
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

    // Bidirectional sync between steps and JSON
    // We use flags to prevent recursive updates
    let isSyncingSteps = false
    let isSyncingJson = false

    // Sync config JSON when steps change (fields → JSON)
    watch(
        steps,
        v => {
            if (isSyncingSteps || isSyncingJson) {
                return
            }
            try {
                const newJson = JSON.stringify({ steps: v }, null, 2)
                // Only update if different to prevent loops
                if (configJson.value !== newJson) {
                    isSyncingJson = true
                    configJson.value = newJson
                    // Reset flag after next tick
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

    // Sync steps when JSON changes manually (JSON → fields)
    watch(configJson, jsonStr => {
        if (isSyncingSteps || isSyncingJson) {
            return
        }
        try {
            const { parsed } = parseJson(jsonStr)
            if (parsed && typeof parsed === 'object' && parsed !== null) {
                const config = parsed as { steps?: unknown[] }
                if (Array.isArray(config.steps)) {
                    isSyncingSteps = true
                    // Sanitize steps when loading from JSON
                    const sanitized = sanitizeDslSteps(config.steps)
                    sanitized.forEach(s => {
                        ensureCsvOptions(s)
                        ensureEntityFilter(s)
                    })
                    steps.value = sanitized
                    // Reset flag after next tick
                    setTimeout(() => {
                        isSyncingSteps = false
                    }, 0)
                }
            }
        } catch {
            // ignore parse errors - user might be typing
        }
    })

    // Expose for tests
    defineExpose({
        submit,
        steps,
        configJson,
        configError,
    })
</script>
