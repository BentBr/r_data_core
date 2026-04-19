import { ref, reactive, onMounted, onUnmounted, defineComponent } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useTranslations } from '@/shared/composables/useTranslations'
import LanguageSwitch from '@/shared/components/LanguageSwitch/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

export default defineComponent({
    name: 'LoginPage',
    components: {
        LanguageSwitch,
        SmartIcon,
    },
    setup() {
        const router = useRouter()
        const authStore = useAuthStore()
        const { t, translateError } = useTranslations()

        const loginForm = ref()
        const formValid = ref(false)
        const showPassword = ref(false)
        const forgotPasswordSnackbar = ref(false)
        const isMobile = ref(false)

        const updateIsMobile = () => {
            isMobile.value = window.innerWidth < 1200
        }

        const credentials = reactive({
            username: '',
            password: '',
        })

        const fieldErrors = reactive<{ username: string[]; password: string[] }>({
            username: [],
            password: [],
        })

        const usernameRules = [
            (v: string) => !!v || t('auth.login.errors.username_required'),
            (v: string) => v.length >= 3 || t('auth.login.errors.username_too_short'),
        ]

        const passwordRules = [
            (v: string) => !!v || t('auth.login.errors.password_required'),
            (v: string) => v.length >= 8 || t('auth.login.errors.password_too_short'),
        ]

        const handleLogin = async () => {
            if (!formValid.value) return
            fieldErrors.username = []
            fieldErrors.password = []
            try {
                await authStore.login(credentials)
                const redirectParam = router.currentRoute.value.query.redirect
                const redirectTo =
                    (Array.isArray(redirectParam) ? redirectParam[0] : redirectParam) ?? '/dashboard'
                void router.push(redirectTo)
            } catch (error) {
                const rawErrorMessage = error instanceof Error ? error.message : t('general.errors.unknown')
                const translatedErrorMessage = translateError(rawErrorMessage)
                const lowerErrorMessage = rawErrorMessage.toLowerCase()
                if (lowerErrorMessage.includes('username') || lowerErrorMessage.includes('user')) {
                    fieldErrors.username = [translatedErrorMessage]
                } else if (lowerErrorMessage.includes('password') || lowerErrorMessage.includes('credential')) {
                    fieldErrors.password = [translatedErrorMessage]
                }
            }
        }

        const clearFieldError = (field: 'username' | 'password') => {
            fieldErrors[field] = []
            authStore.clearError()
        }

        const showForgotPassword = () => {
            forgotPasswordSnackbar.value = true
        }

        onMounted(() => {
            updateIsMobile()
            window.addEventListener('resize', updateIsMobile)
            if (authStore.isAuthenticated) {
                const redirectParam = router.currentRoute.value.query.redirect
                const redirectTo =
                    (Array.isArray(redirectParam) ? redirectParam[0] : redirectParam) ?? '/dashboard'
                void router.push(redirectTo)
            }
        })

        onUnmounted(() => {
            window.removeEventListener('resize', updateIsMobile)
        })

        return {
            t,
            authStore,
            loginForm,
            formValid,
            showPassword,
            forgotPasswordSnackbar,
            isMobile,
            credentials,
            fieldErrors,
            usernameRules,
            passwordRules,
            handleLogin,
            clearFieldError,
            showForgotPassword,
        }
    },
})
