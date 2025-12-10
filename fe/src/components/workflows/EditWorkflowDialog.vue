<template>
    <v-dialog
        v-model="model"
        :max-width="getDialogMaxWidth('wide')"
    >
        <v-card>
            <v-card-title class="pa-6">Edit Workflow</v-card-title>
            <v-card-text class="pa-6">
                <v-tabs v-model="activeTab">
                    <v-tab value="edit">Edit</v-tab>
                    <v-tab value="history">{{ t('entities.details.history') }}</v-tab>
                </v-tabs>

                <v-window v-model="activeTab">
                    <v-window-item value="edit">
                        <v-form
                            ref="formRef"
                            class="mt-4"
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
                                    <v-expansion-panel-title>{{
                                        t('workflows.create.config_label')
                                    }}</v-expansion-panel-title>
                                    <v-expansion-panel-text>
                                        <div class="mb-4">
                                            <DslConfigurator
                                                v-model="steps"
                                                :workflow-uuid="workflowUuid"
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
                    </v-window-item>

                    <v-window-item value="history">
                        <VersionHistory
                            ref="versionHistoryRef"
                            :versions="versions"
                            @compare="handleVersionCompare"
                        />
                    </v-window-item>
                </v-window>
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    variant="text"
                    color="mutedForeground"
                    @click="cancel"
                    >Cancel</v-btn
                >
                <v-btn
                    color="primary"
                    variant="flat"
                    :loading="loading"
                    @click="submit"
                    >Save</v-btn
                >
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { computed, ref, watch } from 'vue'
    import { typedHttpClient } from '@/api/typed-client'
    import { ValidationError } from '@/api/typed-client'
    import { getDialogMaxWidth } from '@/design-system/components'
    import DslConfigurator from './DslConfigurator.vue'
    import WorkflowFormFields from './WorkflowFormFields.vue'
    import VersionHistory from '@/components/common/VersionHistory.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { computeDiffRows } from '@/utils/versionDiff'
    import type { DslStep } from './dsl/dsl-utils'
    import type { WorkflowConfig } from '@/types/schemas/workflow'

    const props = defineProps<{ modelValue: boolean; workflowUuid: string | null }>()
    const emit = defineEmits<{
        (e: 'update:modelValue', value: boolean): void
        (e: 'updated'): void
    }>()

    const { t } = useTranslations()
    const model = computed({ get: () => props.modelValue, set: v => emit('update:modelValue', v) })
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

    // Check if any step has from.api source type (accepts POST, no cron needed)
    const hasApiSource = computed(() => {
        return steps.value.some((step: DslStep) => {
            if (step.from?.type === 'format' && step.from?.source?.source_type === 'api') {
                // from.api without endpoint field = accepts POST
                return !step.from?.source?.config?.endpoint
            }
            return false
        })
    })

    // Check if any step has to.format.output.mode === 'api' (exports via GET, no cron needed)
    const hasApiOutput = computed(() => {
        return steps.value.some((step: DslStep) => {
            if (step.to?.type === 'format') {
                const output = step.to.output
                if (typeof output === 'string') {
                    return output === 'api'
                }
                if (typeof output === 'object' && output !== null && 'mode' in output) {
                    return output.mode === 'api'
                }
            }
            return false
        })
    })

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

    const handleVersionCompare = async (versionA: number, versionB: number) => {
        if (!props.workflowUuid) {
            return
        }
        try {
            const [a, b] = await Promise.all([
                typedHttpClient.getWorkflowVersion(props.workflowUuid, versionA),
                typedHttpClient.getWorkflowVersion(props.workflowUuid, versionB),
            ])
            const diffRows = computeDiffRows(
                (a.data as Record<string, unknown>) ?? {},
                (b.data as Record<string, unknown>) ?? {}
            )
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
            form.value.kind = data.kind
            form.value.enabled = data.enabled
            form.value.schedule_cron = data.schedule_cron ?? ''
            form.value.versioning_disabled =
                'versioning_disabled' in data && typeof data.versioning_disabled === 'boolean'
                    ? data.versioning_disabled
                    : false
            configJson.value = JSON.stringify(data.config ?? {}, null, 2)
            try {
                const cfg = (data.config ?? {}) as { steps?: DslStep[] }
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

    function cancel() {
        model.value = false
    }

    function parseJson(input: string): unknown | undefined {
        if (!input?.trim()) {
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
        // Sync steps to config JSON before validation
        if (steps.value && steps.value.length > 0) {
            isSyncingSteps = true
            configJson.value = JSON.stringify({ steps: steps.value }, null, 2)
            setTimeout(() => {
                isSyncingSteps = false
            }, 0)
        }
        const parsedConfig = parseJson(configJson.value)
        if (parsedConfig === null) {
            configError.value = 'Invalid JSON'
            return
        }
        // Strict DSL presence and validation against BE
        try {
            const config = parsedConfig as { steps?: unknown[] }
            const steps = Array.isArray(config?.steps) ? config.steps : null
            if (!steps || steps.length === 0) {
                configError.value = 'DSL steps are required'
                return
            }
            await typedHttpClient.validateDsl(steps)
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
                    configError.value = e.message ?? 'Invalid DSL'
                }
                return
            }
            if (e && typeof e === 'object' && 'violations' in e) {
                const violations = (e as { violations?: Array<{ message?: string }> }).violations
                if (violations && violations.length > 0) {
                    const v = violations[0]
                    configError.value = v?.message ?? 'Invalid DSL'
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
                description: form.value.description ?? null,
                kind: form.value.kind,
                enabled: form.value.enabled,
                // Set schedule_cron to null when API source or API output is used
                schedule_cron:
                    hasApiSource.value || hasApiOutput.value
                        ? null
                        : (form.value.schedule_cron ?? null),
                config: (parsedConfig ?? {}) as WorkflowConfig,
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

    // Sync config JSON when steps change, but only on explicit updates (not during prop sync)
    // We use a flag to prevent recursive updates
    let isSyncingSteps = false
    watch(
        () => steps.value,
        v => {
            if (isSyncingSteps) {
                return
            }
            try {
                const newJson = JSON.stringify({ steps: v }, null, 2)
                // Only update if different to prevent loops
                if (configJson.value !== newJson) {
                    configJson.value = newJson
                }
            } catch {
                // ignore
            }
        },
        { deep: false } // Shallow watch to prevent deep reactivity issues
    )
</script>

<style scoped></style>
