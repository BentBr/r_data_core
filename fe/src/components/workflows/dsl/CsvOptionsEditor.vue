<template>
    <div class="d-flex ga-2 mb-2 flex-wrap">
        <v-checkbox
            v-model="opts.header"
            :label="t('workflows.dsl.csv_header')"
            color="success"
            density="comfortable"
        />
        <v-text-field
            v-model="opts.delimiter"
            :label="t('workflows.dsl.csv_delimiter')"
            density="comfortable"
            style="max-width: 120px"
        />
        <v-text-field
            v-model="opts.escape"
            :label="t('workflows.dsl.csv_escape')"
            density="comfortable"
            style="max-width: 120px"
        />
        <v-text-field
            v-model="opts.quote"
            :label="t('workflows.dsl.csv_quote')"
            density="comfortable"
            style="max-width: 120px"
        />
    </div>
</template>

<script setup lang="ts">
    import { ref, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'

    type CsvOptions = { header?: boolean; delimiter?: string; escape?: string; quote?: string }
    const props = defineProps<{ modelValue: CsvOptions }>()
    const emit = defineEmits<{ (e: 'update:modelValue', v: CsvOptions): void }>()

    const { t } = useTranslations()

    function defaults(): CsvOptions {
        return { header: true, delimiter: ',', escape: undefined, quote: undefined }
    }

    const opts = ref<CsvOptions>({ ...(props.modelValue || defaults()) })

    watch(
        () => props.modelValue,
        v => {
            opts.value = { ...(v || defaults()) }
        },
        { immediate: true }
    )

    watch(
        () => opts.value,
        v => emit('update:modelValue', { ...v }),
        { deep: true }
    )
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
</style>
