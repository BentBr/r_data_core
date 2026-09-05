// The variable-file rule is named so the template allowance below can waive
// exactly this one rule and nothing else.
const ENV_RULE = /\.env\b/                // .env, .env.dev, .env.test (word boundary)

const sensitivePatterns = [
    ENV_RULE,
    /\.pem$/,                             // JWT keys and certificates
    /\.key$/,                             // Private keys
    /credentials/i,                       // Credential files
    /secrets?\//i,                        // Secrets directories
]

// Broader patterns for Bash commands and Glob/Grep, which can reach files via
// shell globs (e.g. `.env*`, `.en?`) the path patterns above wouldn't catch.
const sensitiveBashPatterns = [...sensitivePatterns, /\.en[?*[]/, /\.e[?*[]/]

// Committed templates that document which variables exist without carrying any
// real value. They must stay editable, otherwise a new setting can never be
// written down where developers look for it.
const templatePattern = /\.env\.(?:example|dist|sample|template)\b/g

// ANCHORED on purpose. An unanchored match would let a path merely *containing*
// the template name through — `.env.example/../.env` reads the real file, and
// `.env.example.local` is a copy that carries real values.
const isTemplatePath = (p) => /\.env\.(?:example|dist|sample|template)$/.test(p)

// For free-form strings (Bash commands, Glob/Grep patterns) the template name
// is removed before matching, so `cat .env.example` passes while
// `cat .env.example .env` still blocks on the second path.
const withoutTemplates = (s) => s.replace(templatePattern, '')

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

        // Check file path for file operations. Being a template waives ONLY the
        // variable-file rule, so a template sitting under secrets/ or
        // credentials/ is still blocked by those rules.
        const isFilePathSensitive = sensitivePatterns.some(
            (p) => p.test(filePath) && !(p === ENV_RULE && isTemplatePath(filePath))
        )

        // Check bash commands for sensitive file access (e.g., docker compose exec ... cat .env)
        const isCommandSensitive =
            toolName === 'Bash' &&
            sensitiveBashPatterns.some((p) => p.test(withoutTemplates(command)))

        // Check Glob/Grep patterns that could enumerate or read secret files
        const isPatternSensitive =
            (toolName === 'Glob' || toolName === 'Grep') &&
            sensitiveBashPatterns.some((p) => p.test(withoutTemplates(pattern)))

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
