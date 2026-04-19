import { defineComponent } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useTranslations } from '@/shared/composables/useTranslations'
import PageLayout from '@/shared/components/PageLayout/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

export default defineComponent({
    name: 'NoAccessPage',
    components: {
        PageLayout,
        SmartIcon,
    },
    setup() {
        const authStore = useAuthStore()
        const { t } = useTranslations()
        return { authStore, t }
    },
})
