const sensitivePatterns = [
    /\.env\b/,                            // .env, .env.dev, .env.test (word boundary)
    /\.pem$/,                             // JWT keys and certificates
    /\.key$/,                             // Private keys
    /credentials/i,                       // Credential files
    /secrets?\//i,                        // Secrets directories
]

// Broader patterns for Bash commands and Glob/Grep, which can reach files via
// shell globs (e.g. `.env*`, `.en?`) the path patterns above wouldn't catch.
const sensitiveBashPatterns = [...sensitivePatterns, /\.en[?*[]/, /\.e[?*[]/]

// Read hook input from stdin (Claude Code passes JSON via stdin)
let input = ''
process.stdin.on('data', (chunk) => {
    input += chunk.toString()
})

process.stdin.on('end', () => {
    try {
        const hookInput = JSON.parse(input)
        const toolName = hookInput.tool_name || ''
        const filePath = hookInput.tool_input?.file_path || hookInput.tool_input?.path || ''
        const command = hookInput.tool_input?.command || ''
        const pattern = hookInput.tool_input?.pattern || ''

        // Check file path for file operations
        const isFilePathSensitive = sensitivePatterns.some((p) => p.test(filePath))

        // Check bash commands for sensitive file access (e.g., docker compose exec ... cat .env)
        const isCommandSensitive = toolName === 'Bash' && sensitiveBashPatterns.some((p) => p.test(command))

        // Check Glob/Grep patterns that could enumerate or read secret files
        const isPatternSensitive =
            (toolName === 'Glob' || toolName === 'Grep') &&
            sensitiveBashPatterns.some((p) => p.test(pattern))

        if (isFilePathSensitive || isCommandSensitive || isPatternSensitive) {
            console.error(
                'Permission denied: Access to sensitive files (.env, keys, credentials, etc.) is blocked by a security hook.'
            )
            process.exit(2) // Exit code 2 blocks the operation
        }

        process.exit(0)
    } catch (err) {
        console.error('Hook error:', err.message)
        process.exit(1)
    }
})
