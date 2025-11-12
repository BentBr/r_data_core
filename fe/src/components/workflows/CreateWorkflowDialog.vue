<template>
    <v-dialog
        v-model="model"
        max-width="1200px"
    >
        <v-card>
            <v-card-title>{{ t('workflows.create.title') }}</v-card-title>
            <v-card-text>
                <v-form
                    ref="formRef"
                    @submit.prevent
                >
                    <v-text-field
                        v-model="form.name"
                        :label="t('workflows.create.name')"
                        :rules="[rules.required]"
                    />
                    <v-textarea
                        v-model="form.description"
                        :label="t('workflows.create.description')"
                        rows="2"
                        auto-grow
                    />
                    <v-select
                        v-model="form.kind"
                        :label="t('workflows.create.kind')"
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
                        :label="t('workflows.create.cron')"
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
                            <v-expansion-panel-title>{{
                                t('workflows.create.config_label')
                            }}</v-expansion-panel-title>
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
            <v-card-actions>
                <v-spacer />
                <v-btn
                    variant="text"
                    @click="cancel"
                    >{{ t('common.cancel') }}</v-btn
                >
                <v-btn
                    color="primary"
                    :loading="loading"
                    @click="submit"
                    >{{ t('workflows.create.create_button') }}</v-btn
                >
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { computed, ref, watch } from 'vue'
    import { typedHttpClient, ValidationError } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import DslConfigurator from './DslConfigurator.vue'

    const props = defineProps<{ modelValue: boolean }>()
    const emit = defineEmits<{
        (e: 'update:modelValue', value: boolean): void
        (e: 'created', uuid: string): void
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

    const rules = {
        required: (v: any) => (!!v && String(v).trim().length > 0) || t('validation.required'),
    }

    function parseJson(input: string): any | undefined {
        if (!input?.trim()) {
            return undefined
        }
        try {
            return JSON.parse(input)
        } catch (e) {
            return null
        }
    }

    function cancel() {
        model.value = false
    }

    async function submit() {
        configError.value = null
        cronError.value = null
        if (steps.value && steps.value.length > 0) {
            configJson.value = JSON.stringify({ steps: steps.value }, null, 2)
        }
        const parsedConfig = parseJson(configJson.value)
        if (parsedConfig === null) {
            configError.value = t('workflows.create.json_invalid')
            return
        }
        // Strict DSL presence and validation against BE
        try {
            const steps = Array.isArray(parsedConfig?.steps) ? parsedConfig.steps : null
            if (!steps || steps.length === 0) {
                configError.value = t('workflows.create.dsl_required')
                return
            }
            await typedHttpClient.validateDsl(steps)
        } catch (e: any) {
            if (e?.violations) {
                const v = e.violations[0]
                configError.value = v?.message || t('workflows.create.dsl_invalid')
                return
            }
            configError.value = e instanceof Error ? e.message : t('workflows.create.dsl_invalid')
            return
        }

        loading.value = true
        try {
            const payload = {
                name: form.value.name,
                description: form.value.description || null,
                kind: form.value.kind,
                enabled: form.value.enabled,
                schedule_cron: form.value.schedule_cron || null,
                config: parsedConfig ?? {},
            }
            const res = await typedHttpClient.createWorkflow(payload)
            emit('created', res.uuid)
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
    // Keep config JSON updated when steps change
    watch(
        steps,
        v => {
            try {
                configJson.value = JSON.stringify({ steps: v }, null, 2)
            } catch {
                // ignore
            }
        },
        { deep: true }
    )
</script>

<script lang="ts">
    export default {
        // Expose for tests
        expose: ['submit', 'steps', 'configJson', 'configError'],
    }
</script>
