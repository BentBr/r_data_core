import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

export const workflowsRoutes: CustomRouteRecord[] = [
    {
        path: '/workflows',
        name: 'Workflows',
        component: () => import('@/modules/workflows/pages/WorkflowsPage/index.vue'),
        meta: { requiresAuth: true, module: 'workflows' },
        navigation: {
            titleKey: 'navigation.workflows',
            icon: 'workflow',
            order: 5,
            visibleInNav: true,
        },
    },
]
