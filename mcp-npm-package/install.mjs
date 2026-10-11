#!/usr/bin/env node
/**
 * Fetch the MCP server binaries for this platform.
 *
 * Runs as a postinstall step. The logic lives in lib/install-core.mjs so it
 * can be tested without touching the network; this file only supplies the
 * real environment: this package's version, the platform, the packaged
 * SHA256SUMS, and `fetch`.
 */

import { readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { install } from './lib/install-core.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))

async function optionalFile(path) {
    try {
        return await readFile(path, 'utf8')
    } catch (error) {
        if (error.code === 'ENOENT') {
            return null
        }
        throw error
    }
}

async function main() {
    const { version } = JSON.parse(await readFile(join(HERE, 'package.json'), 'utf8'))
    await install({
        fetch: globalThis.fetch,
        version,
        platform: process.platform,
        arch: process.arch,
        binDir: join(HERE, 'bin'),
        sumsText: await optionalFile(join(HERE, 'SHA256SUMS')),
        token: process.env.GITHUB_TOKEN,
        log: message => console.log(message),
    })
}

main().catch(error => {
    console.error(`\n@rdatacore/mcp-server install failed:\n${error.message}\n`)
    process.exit(1)
})
