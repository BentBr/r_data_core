import { test, expect, request as pwRequest } from '@playwright/test'
import { clearLoginRateLimit } from '../helpers/redis'

// Verifies the BACKEND's login feedback is safe — correct user-facing wording
// with no leak of internal information. Specifically:
//   * an unknown user and a wrong password return the *same* generic message
//     (no account enumeration), and
//   * error messages never echo the attempted username or reveal account
//     existence / state (locked, inactive, "not found", enum labels, etc.).

const API_BASE_URL = process.env.E2E_API_BASE_URL ?? 'http://rdatacore.docker'
const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? 'admin'
const LOGIN_PATH = '/admin/api/v1/auth/login'

// Words that would betray internal state / account existence if they appeared
// in a login error shown to an unauthenticated caller.
const LEAK_TERMS = [
    'not found',
    'does not exist',
    "doesn't exist",
    'unknown user',
    'no such',
    'locked',
    'inactive',
    'disabled',
    'pending_activation',
    'admin_user_status',
]

test.describe('Login feedback safety', () => {
    test.afterAll(async () => {
        // The probes above bump the per-IP counter; clear it (best-effort).
        await clearLoginRateLimit()
    })

    test('invalid login is generic and leaks no account information', async () => {
        const ctx = await pwRequest.newContext({ baseURL: API_BASE_URL })

        const unknown = await ctx.post(LOGIN_PATH, {
            data: { username: 'e2e_no_such_user', password: 'some_wrong_password' },
        })
        const wrongPassword = await ctx.post(LOGIN_PATH, {
            data: { username: ADMIN_USERNAME, password: 'definitely_the_wrong_password' },
        })

        // Both are rejected with the same status...
        expect(unknown.status()).toBe(401)
        expect(wrongPassword.status()).toBe(401)

        const unknownBody = await unknown.json()
        const wrongBody = await wrongPassword.json()

        // ...and the SAME message — an attacker can't tell "no such user" from
        // "wrong password", so account existence can't be probed.
        expect(wrongBody.message).toBe(unknownBody.message)

        for (const body of [unknownBody, wrongBody]) {
            const message = String(body.message).toLowerCase()
            // Never echoes the attempted username.
            expect(message).not.toContain('e2e_no_such_user')
            expect(message).not.toContain(ADMIN_USERNAME.toLowerCase())
            // Never reveals existence/state or internal enum labels.
            for (const term of LEAK_TERMS) {
                expect(message, `login error must not contain "${term}"`).not.toContain(term)
            }
        }

        await ctx.dispose()
    })
})
