import { authRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const authModule: FrontendModule = {
    key: 'auth',
    routes: authRoutes as CustomRouteRecord[], // Explicitly cast or type
}
