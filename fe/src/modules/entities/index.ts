import { entitiesRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const entitiesModule: FrontendModule = {
    key: 'entities',
    routes: entitiesRoutes as CustomRouteRecord[], // Explicitly cast or type
}
