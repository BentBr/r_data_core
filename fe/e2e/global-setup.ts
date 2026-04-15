import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
    login,
    createEntityDefinition,
    createRole,
    createUser,
    createWorkflow,
    createApiKey,
    type TestDataIds,
} from './helpers/api-client'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const TEST_DATA_FILE = path.resolve(__dirname, '.e2e-test-data.json')

export default async function globalSetup(): Promise<void> {
    console.log('[E2E Setup] Starting global setup...')

    const testData: TestDataIds = {}

    const token = await login()
    console.log('[E2E Setup] Admin login successful')

    try {
        testData.entityDefinitionUuid = await createEntityDefinition(token)
        if (testData.entityDefinitionUuid) {
            console.log(`[E2E Setup] Created entity definition: ${testData.entityDefinitionUuid}`)
        }
    } catch (error) {
        console.warn('[E2E Setup] Entity definition creation failed (non-fatal):', error)
    }

    try {
        testData.roleUuid = await createRole(token)
        if (testData.roleUuid) {
            console.log(`[E2E Setup] Created role: ${testData.roleUuid}`)
        }
    } catch (error) {
        console.warn('[E2E Setup] Role creation failed (non-fatal):', error)
    }

    try {
        testData.userUuid = await createUser(token, testData.roleUuid ?? '')
        if (testData.userUuid) {
            console.log(`[E2E Setup] Created user: ${testData.userUuid}`)
        }
    } catch (error) {
        console.warn('[E2E Setup] User creation failed (non-fatal):', error)
    }

    try {
        testData.workflowUuid = await createWorkflow(token)
        if (testData.workflowUuid) {
            console.log(`[E2E Setup] Created workflow: ${testData.workflowUuid}`)
        }
    } catch (error) {
        console.warn('[E2E Setup] Workflow creation failed (non-fatal):', error)
    }

    try {
        testData.apiKeyUuid = await createApiKey(token)
        if (testData.apiKeyUuid) {
            console.log(`[E2E Setup] Created API key: ${testData.apiKeyUuid}`)
        }
    } catch (error) {
        console.warn('[E2E Setup] API key creation failed (non-fatal):', error)
    }

    fs.writeFileSync(TEST_DATA_FILE, JSON.stringify(testData, null, 2))
    console.log('[E2E Setup] Global setup complete')
}
