const { execFileSync } = require('child_process')
const path = require('path')

let input = ''
process.stdin.on('data', (c) => {
    input += c.toString()
})

process.stdin.on('end', () => {
    try {
        const hookInput = JSON.parse(input)
        const filePath = hookInput.tool_input?.file_path || ''
        if (!filePath.endsWith('.rs')) process.exit(0)

        const rel = path.relative(process.cwd(), filePath)
        try {
            // execFileSync: no shell, so the path is passed as a literal arg.
            execFileSync('cargo', ['fmt', '--', rel], { stdio: 'pipe' })
        } catch {
            // A formatting failure must never block an edit — skip silently.
        }
        process.exit(0)
    } catch (err) {
        console.error('Hook error:', err.message)
        process.exit(1)
    }
})
