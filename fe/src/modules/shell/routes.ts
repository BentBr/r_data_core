import type { RouteRecordRaw } from 'vue-router'

export const shellRoutes: RouteRecordRaw[] = [
    {
        path: '/no-access',
        name: 'NoAccess',
        component: () => import('@/modules/shell/pages/NoAccessPage/index.vue'),
        meta: { requiresAuth: true, module: 'shell' },
    },
]
