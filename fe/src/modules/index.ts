import { apiKeysModule } from './api-keys'
import { authModule } from './auth'
import { dashboardModule } from './dashboard'
import { entitiesModule } from './entities'
import { entityDefinitionsModule } from './entity-definitions'
import { permissionsModule } from './permissions'
import { shellModule } from './shell'
import { systemModule } from './system'
import { workflowsModule } from './workflows'
import type { FrontendModule } from './types'

export const registeredModules: FrontendModule[] = [
    authModule,
    dashboardModule,
    entitiesModule,
    entityDefinitionsModule,
    workflowsModule,
    permissionsModule,
    apiKeysModule,
    systemModule,
    shellModule,
]

export type { FrontendModule } from './types'
