const sensitivePatterns = [
    /\.env/,                              // All .env files
    /\.pem$/,                             // JWT keys and certificates
    /\.key$/,                             // Private keys
    /credentials/i,                       // Credential files
    /secrets?\//i,                        // Secrets directories
]

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

        // Check file path for file operations
        const isFilePathSensitive = sensitivePatterns.some((pattern) => pattern.test(filePath))

        // Check bash commands for sensitive file access (e.g., docker compose exec ... cat .env)
        const isCommandSensitive = toolName === 'Bash' && sensitivePatterns.some((pattern) => pattern.test(command))

        if (isFilePathSensitive || isCommandSensitive) {
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
