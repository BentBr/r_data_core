import type { CustomRouteRecord } from '@/types/router' // Import the custom type

// Removed local interface definitions: NavigationMetadata, ExtendedRouteRecordRaw

export interface FrontendModule {
    key: string
    routes: CustomRouteRecord[] // Changed from ExtendedRouteRecordRaw[]
}
