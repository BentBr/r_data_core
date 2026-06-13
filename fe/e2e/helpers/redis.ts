import net from 'node:net'

const REDIS_HOST = process.env.E2E_REDIS_HOST ?? 'redis'
const REDIS_PORT = Number(process.env.E2E_REDIS_PORT ?? '6379')

// Delete only the per-IP login rate-limit counters (key prefix `login_rl:`).
// A successful login can't clear them (the 429 check short-circuits before
// auth) and they persist for the rate-limit window, so we clear them out of
// band. Surgical via Lua so the shared dev cache/queue in this Redis is left
// intact (unlike FLUSHDB). Integer reply (`:N`) makes completion trivial to detect.
const CLEAR_SCRIPT =
    "local k=redis.call('keys',KEYS[1]); for i=1,#k do redis.call('del',k[i]) end; return #k"

/**
 * Clear all admin-login rate-limit counters from the e2e Redis.
 *
 * Dependency-free (raw RESP over a socket). Used by the global setup/teardown
 * hooks and the rate-limit spec so a triggered 429 never throttles the shared
 * test-runner IP for subsequent logins.
 */
export function clearLoginRateLimit(): Promise<void> {
    return new Promise((resolve, reject) => {
        const socket = net.connect(REDIS_PORT, REDIS_HOST)
        socket.setTimeout(3_000)
        const args = ['EVAL', CLEAR_SCRIPT, '1', 'login_rl:*']
        const command =
            `*${args.length}\r\n` + args.map(a => `$${Buffer.byteLength(a)}\r\n${a}\r\n`).join('')
        socket.on('connect', () => socket.write(command))
        socket.on('data', data => {
            const reply = data.toString()
            // ':N\r\n' = integer (count deleted); '-...' = error.
            if (reply.startsWith(':')) {
                socket.end()
                resolve()
            } else if (reply.startsWith('-')) {
                socket.end()
                reject(new Error(`Redis error clearing rate limit: ${reply.trim()}`))
            }
        })
        socket.on('timeout', () => {
            socket.destroy()
            reject(new Error('Redis rate-limit clear timed out'))
        })
        socket.on('error', reject)
    })
}
