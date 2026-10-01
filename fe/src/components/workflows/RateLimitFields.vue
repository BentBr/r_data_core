<template>
    <v-switch
        :model-value="modelValue.enabled"
        :label="t('workflows.create.rate_limit.enabled')"
        data-testid="rate-limit-enabled"
        color="primary"
        hide-details
        @update:model-value="emitPatch({ enabled: Boolean($event) })"
    />
    <template v-if="modelValue.enabled">
        <v-text-field
            :model-value="modelValue.max_requests"
            type="number"
            min="1"
            :label="t('workflows.create.rate_limit.max_requests')"
            :hint="t('workflows.create.rate_limit.hint')"
            :rules="positiveIntegerRules"
            persistent-hint
            data-testid="rate-limit-max"
            @update:model-value="emitPatch({ max_requests: Number($event) })"
        />
        <v-text-field
            :model-value="modelValue.window_minutes"
            type="number"
            min="1"
            :label="t('workflows.create.rate_limit.window_minutes')"
            :rules="positiveIntegerRules"
            data-testid="rate-limit-window"
            @update:model-value="emitPatch({ window_minutes: Number($event) })"
        />
    </template>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'

    import type { WorkflowRateLimit } from '@/types/schemas/workflow'

    const { t } = useTranslations()

    const props = defineProps<{ modelValue: WorkflowRateLimit }>()
    const emit = defineEmits<{
        (e: 'update:modelValue', value: WorkflowRateLimit): void
    }>()

    // An HTML `min` attribute does not stop a save. The backend reads a zero or
    // a fraction as no limit at all, so without these rules the UI could show
    // "enabled" while nothing is enforced - a silent failure.
    const positiveIntegerRules = [
        (v: unknown) => Number.isInteger(Number(v)) || t('workflows.create.rate_limit.integer'),
        (v: unknown) => Number(v) >= 1 || t('workflows.create.rate_limit.min'),
    ]

    function emitPatch(patch: Partial<WorkflowRateLimit>) {
        emit('update:modelValue', { ...props.modelValue, ...patch })
    }
</script>
