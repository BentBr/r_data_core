import type { RouteRecordRaw } from 'vue-router'

// Define the custom navigation metadata structure
export interface NavigationMetadata {
    titleKey: string;
    icon: string;
    order: number;
    visibleInNav?: boolean; // Optional, defaults to true
}

// Define a type that combines RouteRecordRaw properties with optional navigation metadata
export type CustomRouteRecord = RouteRecordRaw & {
    navigation?: NavigationMetadata;
};
