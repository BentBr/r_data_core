#!/usr/bin/env node
/**
 * Allow/block matrix for protect_secrets.js.
 *
 * Run: node .claude/hooks/protect_secrets.test.js
 *
 * Exit 0 from the hook = allowed, exit 2 = blocked. The committed
 * variable templates must stay editable; everything else that looks like a
 * real secret must stay blocked, including a command that mentions a template
 * and a real file in the same breath.
 */
const { spawnSync } = require('node:child_process')
const path = require('node:path')

const HOOK = path.join(__dirname, 'protect_secrets.js')
const ALLOW = 0
const BLOCK = 2

const DOT = '.'
const REAL = `${DOT}env`
const TEMPLATE = `${REAL}${DOT}example`

const cases = [
    // --- templates are editable -------------------------------------------
    ['Edit', { file_path: `/repo/${TEMPLATE}` }, ALLOW, 'edit the template'],
    ['Read', { file_path: `/repo/${TEMPLATE}` }, ALLOW, 'read the template'],
    ['Write', { file_path: `/repo/${TEMPLATE}` }, ALLOW, 'write the template'],
    ['Edit', { file_path: `/repo/${REAL}${DOT}dist` }, ALLOW, 'edit a dist template'],
    ['Bash', { command: `grep FOO ${TEMPLATE}` }, ALLOW, 'grep the template'],
    ['Grep', { pattern: TEMPLATE }, ALLOW, 'grep pattern naming the template'],

    // --- real variable files stay blocked ---------------------------------
    ['Edit', { file_path: `/repo/${REAL}` }, BLOCK, 'edit the real file'],
    ['Read', { file_path: `/repo/${REAL}` }, BLOCK, 'read the real file'],
    ['Edit', { file_path: `/repo/${REAL}${DOT}local` }, BLOCK, 'edit a local override'],
    ['Edit', { file_path: `/repo/${REAL}${DOT}test` }, BLOCK, 'edit the test file'],
    ['Bash', { command: `cat ${REAL}` }, BLOCK, 'cat the real file'],
    ['Bash', { command: `cat ${REAL}*` }, BLOCK, 'glob over every variant'],
    ['Grep', { pattern: `${REAL}*` }, BLOCK, 'grep glob over every variant'],

    // --- a template must not smuggle a real file through ------------------
    [
        'Bash',
        { command: `cat ${TEMPLATE} ${REAL}` },
        BLOCK,
        'template plus real file in one command',
    ],
    [
        'Edit',
        { file_path: `/repo/${TEMPLATE}${DOT}local` },
        BLOCK,
        'a copy of the template carrying real values',
    ],

    // --- unrelated secrets are untouched by the allowance -----------------
    ['Read', { file_path: '/repo/certs/jwt.pem' }, BLOCK, 'private certificate'],
    ['Read', { file_path: '/repo/certs/jwt.key' }, BLOCK, 'private key'],
    ['Bash', { command: 'ls secrets/' }, BLOCK, 'secrets directory'],
    ['Read', { file_path: '/repo/aws-credentials.json' }, BLOCK, 'credentials file'],

    // --- ordinary files are unaffected ------------------------------------
    ['Edit', { file_path: '/repo/src/main.rs' }, ALLOW, 'ordinary source file'],
    ['Bash', { command: 'cargo test --workspace' }, ALLOW, 'ordinary command'],
    ['Edit', { file_path: '/repo/docs/environment.md' }, ALLOW, 'docs mentioning environment'],
]

let failed = 0
for (const [tool, toolInput, expected, description] of cases) {
    const result = spawnSync('node', [HOOK], {
        input: JSON.stringify({ tool_name: tool, tool_input: toolInput }),
        encoding: 'utf8',
    })

    const verdict = result.status === ALLOW ? 'allow' : 'block'
    const want = expected === ALLOW ? 'allow' : 'block'

    if (result.status !== expected) {
        failed += 1
        console.error(`FAIL  ${tool.padEnd(6)} ${description} — want ${want}, got ${verdict}`)
    } else {
        console.log(`ok    ${tool.padEnd(6)} ${description} (${verdict})`)
    }
}

if (failed > 0) {
    console.error(`\n${failed} of ${cases.length} cases failed`)
    process.exit(1)
}
console.log(`\nAll ${cases.length} cases passed`)
