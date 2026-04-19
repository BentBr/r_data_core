import { defineComponent } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useTheme } from '@/shared/composables/useTheme'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'

export default defineComponent({
    name: 'UserProfileMenu',
    components: {
        SmartIcon,
        Badge,
    },
    setup() {
        const router = useRouter()
        const authStore = useAuthStore()
        const { t } = useTranslations()
        const { isDark, toggleTheme, userPreference } = useTheme()

        const getThemeDisplayName = (): string => {
            switch (userPreference.value) {
                case 'system': return t('general.theme.system')
                case 'light': return t('general.theme.light')
                case 'dark': return t('general.theme.dark')
                default: return t('general.theme.system')
            }
        }

        const handleLogout = async () => {
            authStore.clearAuthState()
            void router.push({ name: 'Login', query: {} })
            try { await authStore.logout() } catch (err) { console.error('Logout failed:', err) }
        }

        return {
            t, authStore, isDark, toggleTheme, getThemeDisplayName,
            goToProfile: () => { console.log('Profile page not yet implemented') },
            handleLogout,
        }
    },
})
