import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

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

    if (requiresAuth) {
        // TODO: Check authentication status
        // For now, allow all routes
        next()
    } else {
        next()
    }
})

export default router
