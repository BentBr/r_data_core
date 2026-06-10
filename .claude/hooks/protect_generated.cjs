const path = require('path')

const GENERATED_DIR = 'fe/src/types/generated/'

const deny = (reason) => {
    console.log(
        JSON.stringify({
            hookSpecificOutput: {
                hookEventName: 'PreToolUse',
                permissionDecision: 'deny',
                permissionDecisionReason: reason,
            },
        }),
    )
    process.exit(0)
}

let input = ''
process.stdin.on('data', (c) => {
    input += c.toString()
})

process.stdin.on('end', () => {
    try {
        const hookInput = JSON.parse(input)
        const filePath = hookInput.tool_input?.file_path || ''
        const rel = path.relative(process.cwd(), filePath)

        if (rel.startsWith(GENERATED_DIR)) {
            deny(
                `${rel} is generated. Do not hand-edit — change the Rust structs and run \`rdt generate-ts\`.`,
            )
            return
        }
        process.exit(0)
    } catch (err) {
        console.error('Hook error:', err.message)
        process.exit(1)
    }
})
