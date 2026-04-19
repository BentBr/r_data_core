import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import { registeredModules } from '@/modules'
import { useAuthStore } from '@/stores/auth'
import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

const staticRoutes: RouteRecordRaw[] = [
    {
        path: '/',
        redirect: '/dashboard',
    },
    {
        path: '/:pathMatch(.*)*',
        name: 'NotFound',
        redirect: '/dashboard',
    },
]

// Explicitly type the routes from modules as CustomRouteRecord
const moduleRoutes: CustomRouteRecord[] = registeredModules.flatMap(module_ => {
    // Ensure module_.routes are typed as CustomRouteRecord.
    // The cast might still be needed if module_.routes is not strictly typed.
    return module_.routes as CustomRouteRecord[];
});

const routes: RouteRecordRaw[] = [
    ...staticRoutes,
    ...moduleRoutes, // Use the typed moduleRoutes
]

const router = createRouter({
    history: createWebHistory(),
    routes,
})

router.beforeEach(async (to, from, next) => {
    // Check if the route requires auth. This uses route.meta.requiresAuth.
    // If route.meta is to be extended with navigation metadata, this part might not be affected.
    const requiresAuth = to.matched.some(record => record.meta.requiresAuth)
    const authStore = useAuthStore()

    if (to.name === 'Login') {
        await authStore.authReady
        if (authStore.isAuthenticated && !authStore.isTokenExpired) {
            next({ name: 'Dashboard' })
            return
        }
        next()
        return
    }

    if (requiresAuth) {
        await authStore.authReady

        if (authStore.isAuthenticated && authStore.isTokenExpired) {
            await authStore.refreshTokens()
        }

        if (!authStore.isAuthenticated) {
            const redirectQuery = from.name === 'Login' ? {} : { redirect: to.fullPath }
            next({
                name: 'Login',
                query: redirectQuery,
            })
            return
        }

        if (authStore.isTokenExpired) {
            await authStore.logout()
            next({
                name: 'Login',
                query: {},
            })
            return
        }

        if (to.path === '/no-access') {
            next()
            return
        }

        const routePath = to.path
        if (!authStore.canAccessRoute(routePath)) {
            const allowedRoutesList = authStore.allowedRoutes as string[]
            if (allowedRoutesList.length > 0) {
                next({
                    path: allowedRoutesList[0],
                })
                return
            }
            next({
                name: 'NoAccess',
            })
            return
        }
    }

    next()
})

export default router
