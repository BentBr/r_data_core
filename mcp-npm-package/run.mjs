#!/usr/bin/env node
/**
 * Run the MCP server binary for this platform.
 *
 * A thin launcher: `stdio: 'inherit'` because stdio mode *is* the protocol
 * stream, and anything this wrapper printed to stdout would corrupt it.
 */

import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = dirname(fileURLToPath(import.meta.url))
const suffix = process.platform === 'win32' ? '.exe' : ''

// HTTP mode is a different binary, so the same launcher serves both and the
// user picks with the variable they were going to set anyway.
const name =
    process.env.RDC_MCP_TRANSPORT === 'http' ? 'r-data-core-mcp-serve' : 'r-data-core-mcp'
const binary = join(HERE, 'bin', `${name}${suffix}`)

if (!existsSync(binary)) {
    console.error(
        `${name} is not installed.\n` +
            `The postinstall step downloads it; if that was skipped ` +
            `(--ignore-scripts, or an offline install), run:\n` +
            `  node ${join(HERE, 'install.mjs')}`
    )
    process.exit(1)
}

const child = spawn(binary, process.argv.slice(2), {
    stdio: 'inherit',
    env: process.env,
})

child.on('error', error => {
    console.error(`could not start ${name}: ${error.message}`)
    process.exit(1)
})

// Pass the child's fate through unchanged: a supervisor reading the exit code
// should see what the server actually did, not what this wrapper made of it.
child.on('exit', (code, signal) => {
    if (signal) {
        process.kill(process.pid, signal)
        return
    }
    process.exit(code ?? 0)
})
