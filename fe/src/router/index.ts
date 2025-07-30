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
        path: '/class-definitions',
        name: 'ClassDefinitions',
        component: () => import('@/pages/class-definitions/ClassDefinitionsPage.vue'),
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
]

const router = createRouter({
    history: createWebHistory(),
    routes,
})

// Navigation guard for authentication
router.beforeEach(async (to, _from, next) => {
    const requiresAuth = to.matched.some(record => record.meta.requiresAuth)
    const authStore = useAuthStore()

    if (requiresAuth) {
        // Check if user is authenticated
        if (!authStore.isAuthenticated) {
            // Redirect to login with return URL
            next({
                name: 'Login',
                query: { redirect: to.fullPath }
            })
            return
        }

        // Check if token is expired
        if (authStore.isTokenExpired) {
            // Token is expired, logout and redirect to login
            authStore.logout()
            next({
                name: 'Login',
                query: { redirect: to.fullPath }
            })
            return
        }
    }

    // If route doesn't require auth OR user is authenticated, proceed
    next()
})

export default router
