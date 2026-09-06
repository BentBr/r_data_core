#!/usr/bin/env node
/**
 * Fetch the MCP server binaries for this platform.
 *
 * Runs as a postinstall step, so the published package stays small and each
 * machine downloads only what it can run.
 *
 * Two things here are not optional. The download is **verified against the
 * release's SHA256SUMS** before anything is made executable — an installer
 * that chmods an unverified download is a supply-chain hole wearing a
 * convenience costume. And an unsupported platform **fails the install
 * loudly** rather than leaving a `bin` entry that fails cryptically the first
 * time someone tries to use it.
 */

import { createHash } from 'node:crypto'
import { chmod, mkdir, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = dirname(fileURLToPath(import.meta.url))
const BIN_DIR = join(HERE, 'bin')
const REPO = 'BentBr/r_data_core'

/** The binaries this package installs. */
const BINARIES = ['r-data-core-mcp', 'r-data-core-mcp-serve']

/**
 * Node's platform/arch pair to the Rust target the release is built for.
 *
 * Listed rather than derived: a wrong guess downloads a binary that cannot
 * run, and the resulting error names neither the platform nor this file.
 */
const TARGETS = {
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin',
    'win32-x64': 'x86_64-pc-windows-msvc',
}

function targetTriple() {
    const key = `${process.platform}-${process.arch}`
    const target = TARGETS[key]
    if (!target) {
        throw new Error(
            `@rdatacore/mcp-server has no binary for ${key}.\n` +
                `Supported: ${Object.keys(TARGETS).join(', ')}.\n` +
                `Build from source instead: cargo build --release -p r_data_core_mcp`
        )
    }
    return target
}

/** Tag prefix for this package's releases. */
const TAG_PREFIX = 'mcp-v'

/**
 * The release to install: a pinned one, or the newest `mcp-v*`.
 *
 * Deliberately not `releases/latest`. That returns the newest release of *any*
 * tag series, and this repository also publishes the server's own releases —
 * so it would hand back a tag carrying none of these assets, and the install
 * would fail with "no asset named ..." rather than anything useful.
 */
function releasePath() {
    const pinned = process.env.RDATACORE_MCP_VERSION
    return pinned
        ? `https://api.github.com/repos/${REPO}/releases/tags/${pinned}`
        : `https://api.github.com/repos/${REPO}/releases?per_page=50`
}

async function fetchRelease() {
    const url = releasePath()
    const headers = { Accept: 'application/vnd.github+json' }
    // Optional, and only useful for the API rate limit — the repository is
    // public, so a token is never required to install.
    if (process.env.GITHUB_TOKEN) {
        headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`
    }

    const response = await fetch(url, { headers })
    if (!response.ok) {
        throw new Error(
            `Could not read the release list from GitHub (${response.status}).\n` +
                `URL: ${url}\n` +
                (response.status === 403
                    ? 'This is usually the unauthenticated API rate limit. Set GITHUB_TOKEN and retry.'
                    : 'Check your network, or pin a known release with RDATACORE_MCP_VERSION.')
        )
    }
    const body = await response.json()
    if (!Array.isArray(body)) {
        return body
    }

    // Newest first is what the API returns; take the first of our own series
    // that is neither a draft nor a prerelease.
    const release = body.find(
        candidate =>
            typeof candidate.tag_name === 'string' &&
            candidate.tag_name.startsWith(TAG_PREFIX) &&
            !candidate.draft &&
            !candidate.prerelease
    )
    if (!release) {
        throw new Error(
            `No published ${TAG_PREFIX}* release among the latest 50 releases of ${REPO}.\n` +
                `Pin one with RDATACORE_MCP_VERSION=${TAG_PREFIX}X.Y.Z if it is older than that.`
        )
    }
    return release
}

function assetUrl(release, name) {
    const asset = (release.assets ?? []).find(candidate => candidate.name === name)
    if (!asset) {
        const available = (release.assets ?? []).map(a => a.name).join(', ') || 'none'
        throw new Error(
            `Release ${release.tag_name} has no asset named ${name}.\n` +
                `Available: ${available}`
        )
    }
    return asset.browser_download_url
}

async function download(url) {
    const response = await fetch(url, { redirect: 'follow' })
    if (!response.ok) {
        throw new Error(`Download failed (${response.status}): ${url}`)
    }
    return Buffer.from(await response.arrayBuffer())
}

/**
 * Parse `SHA256SUMS` into a name → digest map.
 *
 * The format is `<hex>  <name>`, two spaces, as `shasum` and `sha256sum`
 * both emit.
 */
function parseChecksums(text) {
    const sums = new Map()
    for (const line of text.split('\n')) {
        const match = line.trim().match(/^([0-9a-f]{64})\s+\*?(.+)$/i)
        if (match) {
            sums.set(match[2].trim(), match[1].toLowerCase())
        }
    }
    return sums
}

function verify(name, bytes, sums) {
    const expected = sums.get(name)
    if (!expected) {
        throw new Error(
            `SHA256SUMS does not cover ${name}, so the download cannot be verified.\n` +
                `Refusing to install an unverified binary.`
        )
    }
    const actual = createHash('sha256').update(bytes).digest('hex')
    if (actual !== expected) {
        throw new Error(
            `Checksum mismatch for ${name}.\n` +
                `  expected ${expected}\n` +
                `  actual   ${actual}\n` +
                `Refusing to install. This means a corrupted download or a tampered release.`
        )
    }
}

async function main() {
    const target = targetTriple()
    const suffix = process.platform === 'win32' ? '.exe' : ''

    const release = await fetchRelease()
    const sums = parseChecksums(
        (await download(assetUrl(release, 'SHA256SUMS'))).toString('utf8')
    )

    await mkdir(BIN_DIR, { recursive: true })

    for (const binary of BINARIES) {
        const assetName = `${binary}-${target}${suffix}`
        const bytes = await download(assetUrl(release, assetName))

        // Verified before it is written, and written before it is made
        // executable. An unverified file must never become runnable.
        verify(assetName, bytes, sums)

        const destination = join(BIN_DIR, `${binary}${suffix}`)
        await writeFile(destination, bytes)
        if (process.platform !== 'win32') {
            await chmod(destination, 0o755)
        }
        console.log(`installed ${binary} (${release.tag_name}, ${target})`)
    }
}

main().catch(error => {
    console.error(`\n@rdatacore/mcp-server install failed:\n${error.message}\n`)
    process.exit(1)
})
