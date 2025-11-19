<template>
    <div class="d-flex ga-2 mb-2 flex-wrap">
        <v-checkbox
            :model-value="opts.has_header"
            :label="t('workflows.dsl.csv_header')"
            color="success"
            density="comfortable"
            @update:model-value="updateField('has_header', $event)"
        />
        <v-text-field
            :model-value="opts.delimiter"
            :label="t('workflows.dsl.csv_delimiter')"
            density="comfortable"
            style="max-width: 120px"
            @update:model-value="updateField('delimiter', $event)"
        />
        <v-text-field
            :model-value="opts.escape"
            :label="t('workflows.dsl.csv_escape')"
            density="comfortable"
            style="max-width: 120px"
            @update:model-value="updateField('escape', $event)"
        />
        <v-text-field
            :model-value="opts.quote"
            :label="t('workflows.dsl.csv_quote')"
            density="comfortable"
            style="max-width: 120px"
            @update:model-value="updateField('quote', $event)"
        />
    </div>
</template>

<script setup lang="ts">
    import { ref, watch, nextTick } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'

    type CsvOptions = { has_header?: boolean; delimiter?: string; escape?: string; quote?: string }
    const props = defineProps<{ modelValue: CsvOptions }>()
    const emit = defineEmits<{ (e: 'update:modelValue', v: CsvOptions): void }>()

    const { t } = useTranslations()

    function defaults(): CsvOptions {
        return { has_header: true, delimiter: ',', escape: undefined, quote: undefined }
    }

    const opts = ref<CsvOptions>({ ...(props.modelValue || defaults()) })

    // Update local state when prop changes
    watch(
        () => props.modelValue,
        v => {
            const newOpts = { ...(v || defaults()) }
            // Only update if actually different to prevent loops
            const currentStr = JSON.stringify(opts.value)
            const newStr = JSON.stringify(newOpts)
            if (currentStr !== newStr) {
                opts.value = newOpts
            }
        },
        { immediate: true }
    )

    function updateField(field: keyof CsvOptions, value: any) {
        opts.value = { ...opts.value, [field]: value }
        // Use nextTick to batch updates and prevent recursive loops
        void nextTick(() => {
            emit('update:modelValue', { ...opts.value })
        })
    }
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
</style>
