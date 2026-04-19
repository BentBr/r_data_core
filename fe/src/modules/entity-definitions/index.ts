import { entityDefinitionsRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const entityDefinitionsModule: FrontendModule = {
    key: 'entity-definitions',
    routes: entityDefinitionsRoutes as CustomRouteRecord[], // Explicitly cast or type
}
