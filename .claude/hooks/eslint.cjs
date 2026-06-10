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
        const rel = path.relative(process.cwd(), filePath)

        // Only lint frontend .ts/.vue files.
        if (!rel.startsWith('fe/') || (!filePath.endsWith('.ts') && !filePath.endsWith('.vue'))) {
            process.exit(0)
        }

        // The node container runs in fe/, so strip the fe/ prefix for the path.
        const feRel = rel.slice('fe/'.length)

        try {
            // execFileSync with an arg array — no shell, path is a literal arg.
            execFileSync('docker', ['compose', 'exec', '-T', 'node', 'npx', 'eslint', feRel], {
                stdio: 'pipe',
            })
            process.exit(0)
        } catch (err) {
            const out = (err.stdout?.toString() || '') + (err.stderr?.toString() || '')
            // Container not running — skip silently.
            if (
                out.includes('No such service') ||
                out.includes('is not running') ||
                out.includes('Error response from daemon')
            ) {
                process.exit(0)
            }
            console.error(`ESLint found errors in ${rel}:\n${out.trim()}`)
            process.exit(2)
        }
    } catch (err) {
        console.error('Hook error:', err.message)
        process.exit(1)
    }
})
