import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const routes: RouteRecordRaw[] = [
    {
        path: '/',
        redirect: '/dashboard',
    },
    {
        path: '/login',
        name: 'Login',
        component: () => import('@/pages/auth/LoginPage.vue'),
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

    // If going to login page, allow it
    if (to.name === 'Login') {
        next()
        return
    }

    if (requiresAuth) {
        // Check if user is authenticated first
        if (!authStore.isAuthenticated) {
            // Immediately redirect to login
            next({
                name: 'Login',
                query: { redirect: to.fullPath },
            })
            return
        }

        // If authenticated, try to check auth status (this will attempt token refresh if needed)
        try {
            await authStore.checkAuthStatus()
        } catch (err) {
            console.error('[Router] Auth check failed:', err)
        }

        // Check again after potential refresh
        if (!authStore.isAuthenticated) {
            // Redirect to login with return URL
            next({
                name: 'Login',
                query: { redirect: to.fullPath },
            })
            return
        }

        // Check if token is expired after potential refresh
        if (authStore.isTokenExpired) {
            // Token is expired, logout and redirect to login
            await authStore.logout()
            next({
                name: 'Login',
                query: { redirect: to.fullPath },
            })
            return
        }
    }

    // If the route doesn't require auth OR user is authenticated, proceed
    next()
})

export default router
