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
    deleteByPrefix,
    deleteUserByUsername,
    type TestDataIds,
} from './helpers/api-client'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const TEST_DATA_FILE = path.resolve(__dirname, '.e2e-test-data.json')

export default async function globalSetup(): Promise<void> {
    console.log('[E2E Setup] Starting global setup...')

    const testData: TestDataIds = {}

    const token = await login()
    console.log('[E2E Setup] Admin login successful')

    // Clean stale viewer data before creating fresh data.
    // The viewer user references a role by UUID — if the role was re-created
    // with a new UUID the user has a dangling reference and can't log in.
    // Delete user first (references role), then role.
    await deleteUserByUsername(token, 'e2e_viewer_user')
    await deleteByPrefix(token, 'roles', 'e2e_viewer')

    // Create test data — failures here abort the entire suite
    testData.entityDefinitionUuid = await createEntityDefinition(token)
    if (testData.entityDefinitionUuid) {
        console.log(`[E2E Setup] Created entity definition: ${testData.entityDefinitionUuid}`)
    }

    testData.roleUuid = await createRole(token)
    if (testData.roleUuid) {
        console.log(`[E2E Setup] Created role: ${testData.roleUuid}`)
    }

    testData.userUuid = await createUser(token, testData.roleUuid ?? '')
    if (testData.userUuid) {
        console.log(`[E2E Setup] Created user: ${testData.userUuid}`)
    }

    testData.workflowUuid = await createWorkflow(token)
    if (testData.workflowUuid) {
        console.log(`[E2E Setup] Created workflow: ${testData.workflowUuid}`)
    }

    testData.apiKeyUuid = await createApiKey(token)
    if (testData.apiKeyUuid) {
        console.log(`[E2E Setup] Created API key: ${testData.apiKeyUuid}`)
    }

    fs.writeFileSync(TEST_DATA_FILE, JSON.stringify(testData, null, 2))
    console.log('[E2E Setup] Global setup complete')
}
