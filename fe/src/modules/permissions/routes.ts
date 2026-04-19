import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

export const permissionsRoutes: CustomRouteRecord[] = [
    {
        path: '/permissions',
        name: 'Permissions',
        component: () => import('@/modules/permissions/pages/PermissionsPage/index.vue'),
        meta: { requiresAuth: true, module: 'permissions' },
        navigation: {
            titleKey: 'navigation.permissions',
            icon: 'shield',
            order: 6,
            visibleInNav: true,
        },
    },
]
