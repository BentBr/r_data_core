import { defineComponent, ref, computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
// import type { FrontendModule } from '@/modules/types' // Removed unused import
import { registeredModules } from '@/modules'
import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

// Define the structure for navigation items that will be passed to the template
interface NavItem {
    title: string
    icon: string
    path: string
    order: number
}

export default defineComponent({
    name: 'NavigationSidebar',
    components: {
        SmartIcon,
    },
    setup() {
        const route = useRoute()
        const authStore = useAuthStore()
        const { t } = useTranslations()

        const isMobile = ref(false)
        const drawer = ref(true)

        const isRouteVisible = (route: CustomRouteRecord): boolean => { // Use CustomRouteRecord
            if (route.navigation?.visibleInNav === false) {
                return false
            }
            if (route.path === '/' || route.path === '/:pathMatch(.*)*' || route.path === '/login' || route.path === '/no-access') {
                return false
            }
            return true
        }

        const navigationItems = computed<NavItem[]>(() => {
            const allRoutes: CustomRouteRecord[] = registeredModules.flatMap(module_ => {
                // Ensure module_.routes are typed as CustomRouteRecord
                return module_.routes as CustomRouteRecord[]; // Cast to CustomRouteRecord
            });

            const navItems: NavItem[] = []

            allRoutes.forEach((route: CustomRouteRecord) => { // Use CustomRouteRecord
                if (
                    route.navigation &&
                    isRouteVisible(route) &&
                    authStore.canAccessRoute(route.path)
                ) {
                    const title = t(route.navigation.titleKey)
                    navItems.push({
                        title: title,
                        icon: route.navigation.icon,
                        path: route.path,
                        order: route.navigation.order ?? Infinity,
                    })
                }
            })

            navItems.sort((a, b) => a.order - b.order)
            return navItems
        })

        const updateIsMobile = () => {
            isMobile.value = window.innerWidth < 1200
        }

        const toggleNav = () => {
            drawer.value = !drawer.value
        }

        const currentRoute = computed(() => route.path)

        watch(isMobile, (newVal) => {
            if (newVal) {
                drawer.value = false
            } else {
                drawer.value = true
            }
        })

        return {
            authStore,
            t,
            isMobile,
            drawer,
            navigationItems,
            currentRoute,
            toggleNav,
            updateIsMobile,
        }
    },
})
