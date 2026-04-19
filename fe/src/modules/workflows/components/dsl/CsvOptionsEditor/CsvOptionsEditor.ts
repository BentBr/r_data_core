import { ref, watch, nextTick, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'

type CsvOptions = { has_header?: boolean; delimiter?: string; escape?: string; quote?: string }

export default defineComponent({
    name: 'CsvOptionsEditor',
    props: {
        modelValue: { type: Object as PropType<CsvOptions>, required: true },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const opts = ref<CsvOptions>({ ...props.modelValue })
        watch(() => props.modelValue, v => {
            const newOpts = { ...v }
            if (JSON.stringify(opts.value) !== JSON.stringify(newOpts)) opts.value = newOpts
        }, { immediate: true })
        function updateField(field: keyof CsvOptions, value: any) {
            opts.value = { ...opts.value, [field]: value }
            void nextTick(() => { emit('update:modelValue', { ...opts.value }) })
        }
        return { t, opts, updateField }
    },
})
