import { workflowsRoutes } from './routes'
import type { FrontendModule } from '../types'
import type { CustomRouteRecord } from '@/types/router' // Added import

export const workflowsModule: FrontendModule = {
    key: 'workflows',
    routes: workflowsRoutes as CustomRouteRecord[], // Explicitly cast or type
}
