import { shellRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const shellModule: FrontendModule = {
    key: 'shell',
    routes: shellRoutes as CustomRouteRecord[], // Explicitly cast or type
}
