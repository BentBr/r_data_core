<template>
    <v-dialog
        v-model="model"
        max-width="1200px"
    >
        <v-card>
            <v-card-title>Edit Workflow</v-card-title>
            <v-card-text>
                <v-form
                    ref="formRef"
                    @submit.prevent
                >
                    <v-text-field
                        v-model="form.name"
                        label="Name"
                        :rules="[rules.required]"
                    />
                    <v-textarea
                        v-model="form.description"
                        label="Description"
                        rows="2"
                        auto-grow
                    />
                    <v-select
                        v-model="form.kind"
                        label="Kind"
                        :items="kinds"
                        item-title="label"
                        item-value="value"
                    />
                    <v-switch
                        v-model="form.enabled"
                        :label="t('workflows.create.enabled')"
                        color="success"
                        inset
                    ></v-switch>
                    <v-text-field
                        v-model="form.schedule_cron"
                        label="Cron"
                        :error-messages="cronError || ''"
                        :disabled="hasApiSource"
                        :hint="hasApiSource ? t('workflows.create.cron_disabled_for_api_source') : ''"
                        persistent-hint
                        @update:model-value="onCronChange"
                    />
                    <div
                        v-if="cronHelp && !hasApiSource"
                        class="text-caption mb-2"
                    >
                        {{ cronHelp }}
                    </div>
                    <div
                        v-if="nextRuns.length && !hasApiSource"
                        class="text-caption"
                    >
                        Next: {{ nextRuns.join(', ') }}
                    </div>

                    <v-expansion-panels class="mt-2">
                        <v-expansion-panel>
                            <v-expansion-panel-title>Config (JSON)</v-expansion-panel-title>
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
            </v-card-text>
            <v-card-actions>
                <v-spacer />
                <v-btn
                    variant="text"
                    @click="cancel"
                    >Cancel</v-btn
                >
                <v-btn
                    color="primary"
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
    import DslConfigurator from './DslConfigurator.vue'
    import { useTranslations } from '@/composables/useTranslations'

    const props = defineProps<{ modelValue: boolean; workflowUuid: string | null }>()
    const emit = defineEmits<{
        (e: 'update:modelValue', value: boolean): void
        (e: 'updated'): void
    }>()

    const { t } = useTranslations()
    const model = computed({ get: () => props.modelValue, set: v => emit('update:modelValue', v) })
    const loading = ref(false)
    const formRef = ref()

    const kinds = [
        { label: 'Consumer', value: 'consumer' },
        { label: 'Provider', value: 'provider' },
    ]

    const form = ref({
        name: '',
        description: '',
        kind: 'consumer' as 'consumer' | 'provider',
        enabled: true,
        schedule_cron: '' as string | null,
    })

    const configJson = ref('')
    const configError = ref<string | null>(null)
    const steps = ref<any[]>([])
    const cronError = ref<string | null>(null)
    const cronHelp = ref<string>(
        'Use standard 5-field cron (min hour day month dow), e.g. "*/5 * * * *"'
    )
    const nextRuns = ref<string[]>([])
    let cronDebounce: any = null

    // Check if any step has from.api source type (accepts POST, no cron needed)
    const hasApiSource = computed(() => {
        return steps.value.some((step: any) => {
            if (step.from?.type === 'format' && step.from?.source?.source_type === 'api') {
                // from.api without endpoint field = accepts POST
                return !step.from?.source?.config?.endpoint
            }
            return false
        })
    })

    const rules = {
        required: (v: any) => !!v || 'Required',
    }

    watch(
        () => props.modelValue,
        open => {
            if (open) {
                void loadDetails()
            }
        }
    )

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
            configJson.value = JSON.stringify(data.config ?? {}, null, 2)
            try {
                const cfg: any = data.config ?? {}
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
        cronError.value = null
        if (cronDebounce) {
            clearTimeout(cronDebounce)
        }
        if (!value?.trim()) {
            nextRuns.value = []
            return
        }
        cronDebounce = setTimeout(async () => {
            try {
                nextRuns.value = await typedHttpClient.previewCron(value)
            } catch (e) {
                nextRuns.value = []
            }
        }, 350)
    }

    function cancel() {
        model.value = false
    }

    function parseJson(input: string): any | undefined {
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
            const steps = Array.isArray(parsedConfig?.steps) ? parsedConfig.steps : null
            if (!steps || steps.length === 0) {
                configError.value = 'DSL steps are required'
                return
            }
            await typedHttpClient.validateDsl(steps)
        } catch (e: any) {
            if (e?.violations) {
                const v = e.violations[0]
                configError.value = v?.message || 'Invalid DSL'
                return
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
                schedule_cron: form.value.schedule_cron || null,
                config: parsedConfig ?? {},
            })
            emit('updated')
            model.value = false
        } catch (e: any) {
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
