import { strict as assert } from 'node:assert'
import { createHash } from 'node:crypto'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { install, parseChecksums, releasePath } from '../lib/install-core.mjs'

const REPO = 'BentBr/r_data_core'
const TARGET = 'x86_64-unknown-linux-gnu'
const ASSETS = ['r-data-core-mcp', 'r-data-core-mcp-serve'].map(b => `${b}-${TARGET}`)

function sha256(bytes) {
    return createHash('sha256').update(bytes).digest('hex')
}

/** A GitHub API + download stub serving exactly one release. */
function fakeGithub({ tag, bodies }) {
    const requested = []
    const sums = ASSETS.map(name => `${sha256(bodies[name])}  ${name}`).join('\n') + '\n'
    const release = {
        tag_name: tag,
        assets: [...ASSETS, 'SHA256SUMS'].map(name => ({
            name,
            browser_download_url: `https://dl.test/${tag}/${name}`,
        })),
    }
    const fetch = async url => {
        requested.push(String(url))
        if (String(url) === `https://api.github.com/repos/${REPO}/releases/tags/${tag}`) {
            return { ok: true, status: 200, json: async () => release }
        }
        if (String(url).startsWith('https://api.github.com/')) {
            return { ok: false, status: 404, json: async () => ({}) }
        }
        const name = String(url).split('/').pop()
        const body = name === 'SHA256SUMS' ? Buffer.from(sums) : bodies[name]
        return body
            ? { ok: true, status: 200, arrayBuffer: async () => body }
            : { ok: false, status: 404, arrayBuffer: async () => Buffer.alloc(0) }
    }
    return { fetch, requested, sums }
}

test('releasePath pins to the package version, never to a listing', () => {
    assert.equal(
        releasePath({ version: '0.4.16' }),
        `https://api.github.com/repos/${REPO}/releases/tags/mcp-v0.4.16`
    )
    assert.equal(
        releasePath({ version: '0.4.16', pinnedTag: 'mcp-v0.3.0' }),
        `https://api.github.com/repos/${REPO}/releases/tags/mcp-v0.3.0`
    )
})

test('only_the_packaged_version_is_requested', async () => {
    const bodies = Object.fromEntries(ASSETS.map(n => [n, Buffer.from(`bin:${n}`)]))
    const github = fakeGithub({ tag: 'mcp-v0.4.16', bodies })
    const binDir = await mkdtemp(join(tmpdir(), 'rdc-mcp-'))
    try {
        await install({
            fetch: github.fetch,
            version: '0.4.16',
            platform: 'linux',
            arch: 'x64',
            binDir,
            sumsText: github.sums,
            log: () => {},
        })
        assert.equal(
            github.requested[0],
            `https://api.github.com/repos/${REPO}/releases/tags/mcp-v0.4.16`
        )
        assert.ok(
            !github.requested.some(u => u.includes('/releases?')),
            `a release listing was requested: ${github.requested.join(', ')}`
        )
        const written = await readFile(join(binDir, 'r-data-core-mcp'))
        assert.equal(written.toString(), 'bin:r-data-core-mcp-x86_64-unknown-linux-gnu')
    } finally {
        await rm(binDir, { recursive: true, force: true })
    }
})

test('a checksum mismatch refuses to install', async () => {
    const bodies = Object.fromEntries(ASSETS.map(n => [n, Buffer.from(`bin:${n}`)]))
    const github = fakeGithub({ tag: 'mcp-v0.4.16', bodies })
    const tampered = github.sums.replace(/^[0-9a-f]{8}/, 'deadbeef')
    const binDir = await mkdtemp(join(tmpdir(), 'rdc-mcp-'))
    try {
        await assert.rejects(
            install({
                fetch: github.fetch,
                version: '0.4.16',
                platform: 'linux',
                arch: 'x64',
                binDir,
                sumsText: tampered,
                log: () => {},
            }),
            /Checksum mismatch/
        )
    } finally {
        await rm(binDir, { recursive: true, force: true })
    }
})

test('a missing packaged SHA256SUMS fails rather than trusting the release', async () => {
    const bodies = Object.fromEntries(ASSETS.map(n => [n, Buffer.from(`bin:${n}`)]))
    const github = fakeGithub({ tag: 'mcp-v0.4.16', bodies })
    await assert.rejects(
        install({
            fetch: github.fetch,
            version: '0.4.16',
            platform: 'linux',
            arch: 'x64',
            binDir: tmpdir(),
            sumsText: null,
            log: () => {},
        }),
        /SHA256SUMS is missing from the package/
    )
})

test('an unsupported platform fails loudly', async () => {
    await assert.rejects(
        install({
            fetch: async () => ({ ok: false, status: 500 }),
            version: '0.4.16',
            platform: 'freebsd',
            arch: 'x64',
            binDir: tmpdir(),
            sumsText: '',
            log: () => {},
        }),
        /has no binary for freebsd-x64/
    )
})

test('parseChecksums reads the two-space format with optional asterisk', () => {
    const sums = parseChecksums('aa'.repeat(32) + '  one\n' + 'bb'.repeat(32) + ' *two\n')
    assert.equal(sums.get('one'), 'aa'.repeat(32))
    assert.equal(sums.get('two'), 'bb'.repeat(32))
})
