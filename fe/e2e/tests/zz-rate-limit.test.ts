import { test, expect, request as pwRequest } from '@playwright/test'
import { clearLoginRateLimit } from '../helpers/redis'

// End-to-end coverage for the per-IP admin-login rate limit (429).
//
// Constraints this spec works around:
//   * The whole suite runs serially against one backend behind the nginx proxy,
//     so every login shares one effective source IP / rate-limit counter.
//   * Once the limit is hit the counter persists for the window and CANNOT be
//     cleared by a successful login (the 429 check short-circuits before auth).
// Therefore, this spec is named `zz-` so it runs LAST, and clears the rate-limit
// counter before it (deterministic start) and after it. Global teardown also
// clears it before its cleanup login, so a tripped 429 never poisons the suite.

const API_BASE_URL = process.env.E2E_API_BASE_URL ?? 'http://rdatacore.docker'
const MAX_ATTEMPTS = 10
const LOGIN_PATH = '/admin/api/v1/auth/login'

test.describe.serial('Admin login rate limiting', () => {
    test.beforeAll(async () => {
        // Start from a known-clean counter regardless of earlier suite logins.
        await clearLoginRateLimit()
    })

    test.afterAll(async () => {
        // Release the throttle for any later login (belt-and-suspenders;
        // global teardown also clears it).
        await clearLoginRateLimit()
    })

    test('blocks with 429 once the attempt limit is exceeded', async () => {
        const ctx = await pwRequest.newContext({ baseURL: API_BASE_URL })

        // Probe a non-existent user so no real account locks; every failed
        // attempt still increments the per-IP counter.
        for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
            const res = await ctx.post(LOGIN_PATH, {
                data: { username: 'e2e_ghost_user', password: 'wrong_password' },
            })
            expect(res.status(), `attempt ${attempt} is under the limit → 401`).toBe(401)
        }

        const limited = await ctx.post(LOGIN_PATH, {
            data: { username: 'e2e_ghost_user', password: 'wrong_password' },
        })
        expect(limited.status(), 'attempt over the limit → 429').toBe(429)

        await ctx.dispose()
    })
})
