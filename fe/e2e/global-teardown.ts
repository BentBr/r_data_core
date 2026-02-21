import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { login, deleteByPrefix } from './helpers/api-client'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const TEST_DATA_FILE = path.resolve(__dirname, '.e2e-test-data.json')

export default async function globalTeardown(): Promise<void> {
    console.log('[E2E Teardown] Starting global teardown...')

    try {
        const token = await login()

        // Delete all e2e_ prefixed test data
        await deleteByPrefix(token, 'users', 'e2e_')
        await deleteByPrefix(token, 'roles', 'e2e_')
        await deleteByPrefix(token, 'entity-definitions', 'e2e_')

        console.log('[E2E Teardown] Cleanup complete')
    } catch (error) {
        console.error('[E2E Teardown] Cleanup failed (non-fatal):', error)
    }

    // Remove temp data file
    try {
        if (fs.existsSync(TEST_DATA_FILE)) {
            fs.unlinkSync(TEST_DATA_FILE)
        }
    } catch {
        // Ignore file cleanup errors
    }
}
