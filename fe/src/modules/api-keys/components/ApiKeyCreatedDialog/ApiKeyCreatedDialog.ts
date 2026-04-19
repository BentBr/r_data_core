import { defineComponent } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import { getDialogMaxWidth } from '@/design-system/components'

export default defineComponent({
    name: 'ApiKeyCreatedDialog',
    components: {
        SmartIcon,
    },
    props: {
        modelValue: { type: Boolean, required: true },
        apiKey: { type: String, required: true },
    },
    emits: ['update:modelValue', 'copy-success'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const copyApiKey = () => {
            navigator.clipboard.writeText(props.apiKey)
                .then(() => emit('copy-success'))
                .catch(err => console.error('Failed to copy API key:', err))
        }
        return { t, copyApiKey, getDialogMaxWidth, emit }
    },
})
