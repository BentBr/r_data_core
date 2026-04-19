import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

export const entityDefinitionsRoutes: CustomRouteRecord[] = [
    {
        path: '/entity-definitions',
        name: 'EntityDefinitions',
        component: () => import('@/modules/entity-definitions/pages/EntityDefinitionsPage/index.vue'),
        meta: { requiresAuth: true, module: 'entity-definitions' },
        navigation: {
            titleKey: 'navigation.entity_definitions',
            icon: 'folder-tree',
            order: 2,
            visibleInNav: true,
        },
    },
]
