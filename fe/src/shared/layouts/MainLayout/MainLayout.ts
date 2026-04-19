import { computed, onMounted, onUnmounted, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useVersionStore } from '@/stores/versions'
import { useTranslations } from '@/shared/composables/useTranslations'
import LanguageSwitch from '@/shared/components/LanguageSwitch/index.vue'
import UserProfileMenu from '@/shared/components/UserProfileMenu/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import DefaultPasswordBanner from '@/shared/components/DefaultPasswordBanner/index.vue'
import LicenseBanner from '@/shared/components/LicenseBanner/index.vue'
import MobileWarningBanner from '@/shared/components/MobileWarningBanner/index.vue'
import NavigationSidebar from '@/shared/components/NavigationSidebar/index.vue' // Import the new component

export default defineComponent({
    name: 'MainLayout',
    components: {
        LanguageSwitch,
        UserProfileMenu,
        SmartIcon,
        DefaultPasswordBanner,
        LicenseBanner,
        MobileWarningBanner,
        NavigationSidebar, // Register the new component
    },
    setup() {
        const route = useRoute()
        const authStore = useAuthStore()
        const versionStore = useVersionStore()
        const { t } = useTranslations()

        // Removed: isMobile, drawer, updateIsMobile, toggleNav, navigationItems computed property

        // currentPageTitle might need to be re-evaluated.
        // If it relied on navigationItems, it might need to access route.meta or similar.
        // For now, keeping it simple, assuming route.name or similar might be available.
        // Or, if the NavigationSidebar passes down its active item's title.
        // Let's assume for now the route itself has some title info or name.
        const currentPageTitle = computed(() => {
            // If route.meta.titleKey exists, use it. Otherwise, fallback.
            // This part needs to be aligned with new structure.
            // For now, using route name as a fallback.
            return route.name?.toString() ?? 'R Data Core'
        })

        // onMounted and onUnmounted might still be needed for other reasons,
        // e.g., if MainLayout itself manages some window events.
        // If they are purely for isMobile/drawer, they can be removed.
        // For now, assuming they might be used for other purposes, I'll keep them but comment out the isMobile logic.
        onMounted(() => {
            // updateIsMobile() // This logic is now in NavigationSidebar
            // window.addEventListener('resize', updateIsMobile)
        })

        onUnmounted(() => {
            // window.removeEventListener('resize', updateIsMobile)
        })

        return {
            authStore, versionStore, t, currentPageTitle,
            // Removed: isMobile, drawer, navigationItems, toggleNav, updateIsMobile
        }
    },
})
