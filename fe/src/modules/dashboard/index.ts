import { dashboardRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const dashboardModule: FrontendModule = {
    key: 'dashboard',
    routes: dashboardRoutes as CustomRouteRecord[], // Explicitly cast or type
}
