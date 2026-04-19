import { apiKeysRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const apiKeysModule: FrontendModule = {
    key: 'api-keys',
    routes: apiKeysRoutes as CustomRouteRecord[], // Explicitly cast or type
}
