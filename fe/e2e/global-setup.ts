import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { login, createEntityDefinition, createRole, createUser, type TestDataIds } from './helpers/api-client'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const TEST_DATA_FILE = path.resolve(__dirname, '.e2e-test-data.json')

export default async function globalSetup(): Promise<void> {
    console.log('[E2E Setup] Starting global setup...')

    const testData: TestDataIds = {}

    try {
        const token = await login()
        console.log('[E2E Setup] Admin login successful')

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

        fs.writeFileSync(TEST_DATA_FILE, JSON.stringify(testData, null, 2))
        console.log('[E2E Setup] Global setup complete')
    } catch (error) {
        console.error('[E2E Setup] Global setup failed:', error)
        // Write partial data so teardown can still clean up
        fs.writeFileSync(TEST_DATA_FILE, JSON.stringify(testData, null, 2))
        throw error
    }
}
