import { permissionsRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const permissionsModule: FrontendModule = {
    key: 'permissions',
    routes: permissionsRoutes as CustomRouteRecord[], // Explicitly cast or type
}
