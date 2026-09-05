import net from 'node:net'

const REDIS_HOST = process.env.E2E_REDIS_HOST ?? 'redis'
const REDIS_PORT = Number(process.env.E2E_REDIS_PORT ?? '6379')

// Delete only the per-IP auth rate-limit counters (`login_rl:` / `register_rl:`).
// A successful login can't clear them (the 429 check short-circuits before
// auth) and they persist for the rate-limit window, so we clear them out of
// band. Surgical via Lua so the shared dev cache/queue in this Redis is left
// intact (unlike FLUSHDB). Integer reply (`:N`) makes completion trivial to detect.
const CLEAR_SCRIPT =
    "local n=0; for _,p in ipairs(KEYS) do local k=redis.call('keys',p); " +
    "for i=1,#k do redis.call('del',k[i]); n=n+1 end end; return n"

/**
 * Clear all admin-login rate-limit counters from the e2e Redis.
 *
 * Dependency-free (raw RESP over a socket). BEST-EFFORT: any connection error
 * (e.g. Redis not reachable under `E2E_REDIS_HOST` in CI) is swallowed so it can
 * never abort global setup/teardown. Locally (compose) it reaches `redis` and
 * clears the counter; in CI each run gets a fresh Redis so a no-op is harmless.
 */
export function clearLoginRateLimit(): Promise<void> {
    return new Promise(resolve => {
        const socket = net.connect(REDIS_PORT, REDIS_HOST)
        socket.setTimeout(3_000)
        const done = (warn?: string): void => {
            if (warn) console.warn(`[E2E] login rate-limit clear skipped: ${warn}`)
            socket.destroy()
            resolve()
        }
        const args = ['EVAL', CLEAR_SCRIPT, '2', 'login_rl:*', 'register_rl:*']
        const command =
            `*${args.length}\r\n` + args.map(a => `$${Buffer.byteLength(a)}\r\n${a}\r\n`).join('')
        socket.on('connect', () => socket.write(command))
        socket.on('data', data => {
            const reply = data.toString()
            // ':N\r\n' = integer (count deleted); '-...' = a Redis error reply.
            if (reply.startsWith(':')) done()
            else if (reply.startsWith('-')) done(`redis replied ${reply.trim()}`)
        })
        socket.on('timeout', () => done('timed out'))
        socket.on('error', e => done(e.message))
    })
}
