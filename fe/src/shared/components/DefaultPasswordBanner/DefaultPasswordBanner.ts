import { defineComponent } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useTranslations } from '@/shared/composables/useTranslations'
import DismissableBanner from '@/shared/components/DismissableBanner/index.vue'

export default defineComponent({
    name: 'DefaultPasswordBanner',
    components: {
        DismissableBanner,
    },
    setup() {
        const authStore = useAuthStore()
        const { t } = useTranslations()
        return { authStore, t, handleDismiss: () => { authStore.dismissDefaultPasswordBanner() } }
    },
})
