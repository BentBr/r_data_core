import { computed, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import MainLayout from '@/shared/layouts/MainLayout/index.vue'
import LoginLayout from '@/shared/layouts/LoginLayout/index.vue'
import { useAuthStore } from '@/stores/auth'
import { useTheme } from '@/shared/composables/useTheme'

export default defineComponent({
    name: 'App',
    components: {
        MainLayout,
        LoginLayout,
    },
    setup() {
        const route = useRoute()
        const authStore = useAuthStore()

        // Initialize theme system
        const {} = useTheme()

        // Determine which layout to use based on route and auth status
        const layoutComponent = computed(() => {
            // Login page uses login layout
            if (route.name === 'Login' || !authStore.isAuthenticated) {
                return LoginLayout
            }

            // All other authenticated pages use the main layout
            return MainLayout
        })

        // Create a unique key for the layout to ensure proper transitions
        const layoutKey = computed(() => {
            // Use route path instead of name to be more specific
            return route.path
        })

        return {
            layoutComponent,
            layoutKey,
            route,
        }
    },
})
