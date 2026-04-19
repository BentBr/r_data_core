import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

export const dashboardRoutes: CustomRouteRecord[] = [
    {
        path: '/dashboard',
        name: 'Dashboard',
        component: () => import('@/modules/dashboard/pages/DashboardPage/index.vue'),
        meta: { requiresAuth: true, module: 'dashboard' },
        navigation: {
            titleKey: 'navigation.dashboard',
            icon: 'layout-dashboard',
            order: 1,
            visibleInNav: true,
        },
    },
]
