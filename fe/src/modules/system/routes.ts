import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

export const systemRoutes: CustomRouteRecord[] = [
    {
        path: '/system',
        name: 'System',
        component: () => import('@/modules/system/pages/SystemPage/index.vue'),
        meta: { requiresAuth: true, module: 'system' },
        navigation: {
            titleKey: 'navigation.system',
            icon: 'settings',
            order: 7,
            visibleInNav: true,
        },
    },
]
