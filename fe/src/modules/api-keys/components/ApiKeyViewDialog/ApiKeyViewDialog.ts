import { defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import { getDialogMaxWidth } from '@/design-system/components'
import type { ApiKey } from '@/types/schemas'

export default defineComponent({
    name: 'ApiKeyViewDialog',
    components: {
        SmartIcon,
    },
    props: {
        modelValue: { type: Boolean, required: true },
        apiKey: { type: Object as PropType<ApiKey | null>, default: null },
    },
    emits: ['update:modelValue'],
    setup() {
        const { t } = useTranslations()
        const formatDate = (dateString: string | null): string => dateString ? new Date(dateString).toLocaleString() : 'Never'
        return { t, formatDate, getDialogMaxWidth }
    },
})
