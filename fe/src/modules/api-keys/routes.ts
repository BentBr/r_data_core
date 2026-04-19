import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

export const apiKeysRoutes: CustomRouteRecord[] = [
    {
        path: '/api-keys',
        name: 'ApiKeys',
        component: () => import('@/modules/api-keys/pages/ApiKeysPage/index.vue'),
        meta: { requiresAuth: true, module: 'api-keys' },
        navigation: {
            titleKey: 'navigation.api_keys',
            icon: 'key',
            order: 4,
            visibleInNav: true,
        },
    },
]
