import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

export const entitiesRoutes: CustomRouteRecord[] = [
    {
        path: '/entities',
        name: 'Entities',
        component: () => import('@/modules/entities/pages/EntitiesPage/index.vue'),
        meta: { requiresAuth: true, module: 'entities' },
        navigation: {
            titleKey: 'navigation.entities',
            icon: 'database',
            order: 3,
            visibleInNav: true,
        },
    },
]
