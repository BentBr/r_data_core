import type { RouteRecordRaw } from 'vue-router'

export const authRoutes: RouteRecordRaw[] = [
    {
        path: '/login',
        name: 'Login',
        component: () => import('@/modules/auth/pages/LoginPage/index.vue'),
        meta: { requiresAuth: false, module: 'auth' },
    },
]
