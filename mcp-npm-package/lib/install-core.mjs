/**
 * The installer's logic, with every side effect injectable.
 *
 * `install.mjs` is the postinstall entry point and wires in the real `fetch`,
 * platform and package version; this module is what the tests exercise.
 *
 * Two things here are not optional. The release is **the one this package
 * version was published for** — `mcp-v${version}` — never "the newest", so a
 * lockfile means what it says and a later release cannot change what an
 * existing install downloads. And every binary is **verified against the
 * `SHA256SUMS` shipped inside this npm package**, which npm's own provenance
 * covers; a checksum file fetched from the same release as the binaries would
 * only prove the two were uploaded together.
 */

import { createHash } from 'node:crypto'
import { chmod, mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

export const REPO = 'BentBr/r_data_core'
export const TAG_PREFIX = 'mcp-v'
export const BINARIES = ['r-data-core-mcp', 'r-data-core-mcp-serve']

/** Node's platform/arch pair to the Rust target the release is built for. */
export const TARGETS = {
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'linux-arm64': 'aarch64-unknown-linux-gnu',
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin',
    'win32-x64': 'x86_64-pc-windows-msvc',
}

export function targetTriple(platform, arch) {
    const key = `${platform}-${arch}`
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

/**
 * The release to install: this package's own version, or an explicit pin.
 *
 * Never a listing. "Newest mcp-v*" would let a later release — or a
 * tampered one — replace what `npm install` of a given version downloads.
 */
export function releasePath({ version, pinnedTag }) {
    const tag = pinnedTag || `${TAG_PREFIX}${version}`
    return `https://api.github.com/repos/${REPO}/releases/tags/${tag}`
}

export async function fetchRelease({ fetch, version, pinnedTag, token }) {
    const url = releasePath({ version, pinnedTag })
    const headers = { Accept: 'application/vnd.github+json' }
    if (token) {
        headers.Authorization = `Bearer ${token}`
    }
    const response = await fetch(url, { headers })
    if (!response.ok) {
        throw new Error(
            `Could not read release ${url.split('/').pop()} from GitHub (${response.status}).\n` +
                `URL: ${url}\n` +
                (response.status === 404
                    ? 'The release for this package version does not exist. The npm package is published only after its binaries, so this usually means a mismatched RDATACORE_MCP_VERSION.'
                    : response.status === 403
                      ? 'This is usually the unauthenticated API rate limit. Set GITHUB_TOKEN and retry.'
                      : 'Check your network and retry.')
        )
    }
    return response.json()
}

export function assetUrl(release, name) {
    const asset = (release.assets ?? []).find(candidate => candidate.name === name)
    if (!asset) {
        const available = (release.assets ?? []).map(a => a.name).join(', ') || 'none'
        throw new Error(
            `Release ${release.tag_name} has no asset named ${name}.\nAvailable: ${available}`
        )
    }
    return asset.browser_download_url
}

async function download(fetch, url) {
    const response = await fetch(url, { redirect: 'follow' })
    if (!response.ok) {
        throw new Error(`Download failed (${response.status}): ${url}`)
    }
    return Buffer.from(await response.arrayBuffer())
}

/** Parse `SHA256SUMS` (`<hex>  <name>`, as `shasum`/`sha256sum` emit). */
export function parseChecksums(text) {
    const sums = new Map()
    for (const line of text.split('\n')) {
        const match = line.trim().match(/^([0-9a-f]{64})\s+\*?(.+)$/i)
        if (match) {
            sums.set(match[2].trim(), match[1].toLowerCase())
        }
    }
    return sums
}

export function verify(name, bytes, sums) {
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
            `Checksum mismatch for ${name}.\n  expected ${expected}\n  actual   ${actual}\n` +
                `Refusing to install. This means a corrupted download or a tampered release.`
        )
    }
}

/**
 * Download, verify and install both binaries.
 *
 * `sumsText` is the content of the `SHA256SUMS` shipped in this package. Its
 * absence is an error, not a reason to trust the release's copy.
 */
export async function install({ fetch, version, pinnedTag, platform, arch, binDir, sumsText, log, token }) {
    const target = targetTriple(platform, arch)
    const suffix = platform === 'win32' ? '.exe' : ''

    if (sumsText === null || sumsText === undefined) {
        throw new Error(
            'SHA256SUMS is missing from the package, so no download can be verified.\n' +
                'Refusing to install. Reinstall the package; if this persists, the published tarball is incomplete.'
        )
    }
    const sums = parseChecksums(sumsText)

    const release = await fetchRelease({ fetch, version, pinnedTag, token })
    await mkdir(binDir, { recursive: true })

    for (const binary of BINARIES) {
        const assetName = `${binary}-${target}${suffix}`
        const bytes = await download(fetch, assetUrl(release, assetName))
        // Verified before it is written, and written before it is made
        // executable. An unverified file must never become runnable.
        verify(assetName, bytes, sums)
        const destination = join(binDir, `${binary}${suffix}`)
        await writeFile(destination, bytes)
        if (platform !== 'win32') {
            await chmod(destination, 0o755)
        }
        log(`installed ${binary} (${release.tag_name}, ${target})`)
    }
}
