import { systemRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const systemModule: FrontendModule = {
    key: 'system',
    routes: systemRoutes as CustomRouteRecord[], // Explicitly cast or type
}
