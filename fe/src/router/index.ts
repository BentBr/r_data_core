import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useCapabilitiesStore } from '@/stores/capabilities'

const routes: RouteRecordRaw[] = [
    {
        path: '/',
        redirect: '/dashboard',
    },
    {
        path: '/workflows',
        name: 'Workflows',
        component: () => import('@/pages/workflows/WorkflowsPage.vue'),
        meta: { requiresAuth: true },
    },
    {
        path: '/login',
        name: 'Login',
        component: () => import('@/pages/auth/LoginPage.vue'),
        meta: { requiresAuth: false },
    },
    {
        path: '/reset-password',
        name: 'ResetPassword',
        component: () => import('@/pages/auth/ResetPasswordPage.vue'),
        meta: { requiresAuth: false },
    },
    {
        path: '/dashboard',
        name: 'Dashboard',
        component: () => import('@/pages/dashboard/DashboardPage.vue'),
        meta: { requiresAuth: true },
    },
    {
        path: '/entity-definitions',
        name: 'EntityDefinitions',
        component: () => import('@/pages/entity-definitions/EntityDefinitionsPage.vue'),
        meta: { requiresAuth: true },
    },
    {
        path: '/entities',
        name: 'Entities',
        component: () => import('@/pages/entities/EntitiesPage.vue'),
        meta: { requiresAuth: true },
    },
    {
        path: '/api-keys',
        name: 'ApiKeys',
        component: () => import('@/pages/api-keys/ApiKeysPage.vue'),
        meta: { requiresAuth: true },
    },
    {
        path: '/permissions',
        name: 'Permissions',
        component: () => import('@/pages/permissions/PermissionsPage.vue'),
        meta: { requiresAuth: true },
    },
    {
        path: '/system',
        name: 'System',
        component: () => import('@/pages/system/SystemPage.vue'),
        meta: { requiresAuth: true },
    },
    {
        path: '/no-access',
        name: 'NoAccess',
        component: () => import('@/pages/no-access/NoAccessPage.vue'),
        meta: { requiresAuth: true },
    },
    // Catch-all route for 404 handling
    {
        path: '/:pathMatch(.*)*',
        name: 'NotFound',
        redirect: '/dashboard',
    },
]

const router = createRouter({
    history: createWebHistory(),
    routes,
})

// Navigation guard for authentication
router.beforeEach(async (to, from, next) => {
    const requiresAuth = to.matched.some(record => record.meta.requiresAuth)
    const authStore = useAuthStore()

    // If going to login page, redirect authenticated users to dashboard
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
        // Wait for the initial auth check to complete. On a full page reload
        // the in-memory access token is lost; this ensures the store has had a
        // chance to restore it from the refresh-token cookie before we decide.
        await authStore.authReady

        // If the token is expired but the user was authenticated, try to refresh
        if (authStore.isAuthenticated && authStore.isTokenExpired) {
            await authStore.refreshTokens()
        }

        // Ensure capabilities are loaded (e.g. after page reload)
        // Wrapped in try-catch: capabilities are non-critical for navigation
        if (authStore.isAuthenticated) {
            try {
                const caps = useCapabilitiesStore()
                if (!caps.isLoaded) {
                    void caps.fetchCapabilities()
                }
            } catch {
                // Pinia may not be active in test environments
            }
        }

        // After potential restore / refresh, check authentication
        if (!authStore.isAuthenticated) {
            // If coming from login page, don't add redirect query (prevents loops after logout)
            // Otherwise, preserve the redirect query for normal auth redirects
            const redirectQuery = from.name === 'Login' ? {} : { redirect: to.fullPath }
            next({
                name: 'Login',
                query: redirectQuery,
            })
            return
        }

        // If token is still expired after refresh attempt, force logout
        if (authStore.isTokenExpired) {
            // Token is expired, logout and redirect to login without redirect query
            await authStore.logout()
            next({
                name: 'Login',
                query: {},
            })
            return
        }

        // Allow access to /no-access route for authenticated users
        if (to.path === '/no-access') {
            next()
            return
        }

        // Check route permissions
        const routePath = to.path
        if (!authStore.canAccessRoute(routePath)) {
            // User doesn't have permission for this route
            // Try to redirect to first available route from allowedRoutes
            // allowedRoutes is exported as readonly ref, access via .value if needed
            const allowedRoutesList = authStore.allowedRoutes as string[]
            if (allowedRoutesList.length > 0) {
                // Redirect to first available route
                next({
                    path: allowedRoutesList[0],
                })
                return
            }
            // If no allowed routes, redirect to /no-access (keep user authenticated)
            next({
                name: 'NoAccess',
            })
            return
        }
    }

    // If the route doesn't require auth OR user is authenticated, proceed
    next()
})

export default router
