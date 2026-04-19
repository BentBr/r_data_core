import { computed, ref, onMounted, onUnmounted, defineComponent } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useTranslations } from '@/shared/composables/useTranslations'
import DismissableBanner from '@/shared/components/DismissableBanner/index.vue'

export default defineComponent({
    name: 'MobileWarningBanner',
    components: {
        DismissableBanner,
    },
    setup() {
        const authStore = useAuthStore()
        const { t } = useTranslations()
        const isMobile = ref(false)

        const updateIsMobile = () => { isMobile.value = window.innerWidth < 1200 }
        const shouldShowBanner = computed(() => isMobile.value && !authStore.isMobileWarningDismissed)
        const handleDismiss = () => { authStore.dismissMobileWarningBanner() }

        onMounted(() => {
            updateIsMobile()
            window.addEventListener('resize', updateIsMobile)
        })

        onUnmounted(() => { window.removeEventListener('resize', updateIsMobile) })

        return { t, shouldShowBanner, handleDismiss }
    },
})
