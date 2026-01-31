<template>
    <div>
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
        <v-switch
            v-model="form.versioning_disabled"
            :label="
                t('workflows.create.versioning_disabled') || 'Disable versioning for this workflow'
            "
            color="warning"
            inset
        ></v-switch>
        <v-text-field
            v-model="form.schedule_cron"
            :label="t('workflows.create.cron')"
            :error-messages="cronError || ''"
            :disabled="hasApiSource || hasApiOutput"
            :hint="
                hasApiSource
                    ? t('workflows.create.cron_disabled_for_api_source')
                    : hasApiOutput
                      ? t('workflows.create.cron_disabled_for_api_output')
                      : ''
            "
            persistent-hint
            @update:model-value="onCronChange"
        />
        <div
            v-if="cronHelp && !hasApiSource && !hasApiOutput"
            class="text-caption mb-2"
        >
            {{ cronHelp }}
        </div>
        <div
            v-if="nextRuns.length && !hasApiSource && !hasApiOutput"
            class="text-caption"
        >
            Next: {{ nextRuns.join(', ') }}
        </div>
    </div>
</template>

<script setup lang="ts">
    import { computed, watch } from 'vue'
    import { typedHttpClient } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import type { DslStep } from './dsl/dsl-utils'

    type WorkflowForm = {
        name: string
        description: string
        kind: 'consumer' | 'provider'
        enabled: boolean
        schedule_cron: string | null
        versioning_disabled: boolean
    }

    const props = defineProps<{
        form: WorkflowForm
        steps: DslStep[]
        cronError: string | null
        cronHelp: string
        nextRuns: string[]
    }>()

    const emit = defineEmits<{
        (e: 'update:form', value: WorkflowForm): void
        (e: 'update:cronError', value: string | null): void
        (e: 'update:nextRuns', value: string[]): void
        (e: 'cronChange', value: string): void
    }>()

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
        // Skip validation if API source is used
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
</script>
