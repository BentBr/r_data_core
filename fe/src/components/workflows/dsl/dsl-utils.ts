/**
 * Utility functions for DSL configuration
 */

export type Mapping = Record<string, string>

export type CsvOptions = {
    has_header?: boolean
    delimiter?: string
    escape?: string
    quote?: string
}

// Auth configuration types
export type KeyLocation = 'header' | 'body'

export type AuthConfig =
    | { type: 'none' }
    | { type: 'api_key'; key: string; header_name?: string }
    | { type: 'basic_auth'; username: string; password: string }
    | { type: 'pre_shared_key'; key: string; location: KeyLocation; field_name: string }

// Source configuration
export type SourceConfig = {
    source_type: string // "uri", "file", "api", "sftp", etc.
    config: Record<string, unknown> // Source-specific config
    auth?: AuthConfig
}

// Format configuration
export type FormatConfig = {
    format_type: string // "csv", "json", "xml", etc.
    options?: Record<string, unknown> // Format-specific options
}

// Destination configuration
export type DestinationConfig = {
    destination_type: string // "uri", "file", "sftp", etc.
    config: Record<string, unknown> // Destination-specific config
    auth?: AuthConfig
}

// HTTP method
export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE' | 'HEAD' | 'OPTIONS'

// Output mode
export type OutputMode =
    | { mode: 'download' }
    | { mode: 'api' }
    | { mode: 'push'; destination: DestinationConfig; method?: HttpMethod }

export type FromDef =
    | { type: 'format'; source: SourceConfig; format: FormatConfig; mapping: Mapping }
    | {
          type: 'entity'
          entity_definition: string
          filter?: { field: string; operator?: string; value: string }
          mapping: Mapping
      }

export type OperandField = { kind: 'field'; field: string }
export type OperandConst = { kind: 'const'; value: number }
export type Operand = OperandField | OperandConst

export type StringOperandField = { kind: 'field'; field: string }
export type StringOperandConst = { kind: 'const_string'; value: string }
export type StringOperand = StringOperandField | StringOperandConst

export type Transform =
    | { type: 'none' }
    | {
          type: 'arithmetic'
          target: string
          left: Operand
          op: 'add' | 'sub' | 'mul' | 'div'
          right: Operand
      }
    | {
          type: 'concat'
          target: string
          left: StringOperand
          separator?: string
          right: StringOperand
      }

export type ToDef =
    | { type: 'format'; output: OutputMode; format: FormatConfig; mapping: Mapping }
    | {
          type: 'entity'
          entity_definition: string
          path: string
          mode: 'create' | 'update' | 'create_or_update'
          update_key?: string
          identify?: { field: string; value: string }
          mapping: Mapping
      }

export type DslStep = { from: FromDef; transform: Transform; to: ToDef }

/**
 * Sanitizes a DSL step by removing invalid fields
 * - Removes 'output' field from entity type 'to' definitions
 * - Ensures required fields exist
 */
export function sanitizeDslStep(step: unknown): DslStep {
    const sanitized = { ...(step as Record<string, unknown>) } as Record<string, unknown>

    // Sanitize 'to' definition
    if (sanitized.to) {
        if (sanitized.to.type === 'entity') {
            // Remove 'output' field from entity type
            const { output, ...rest } = sanitized.to
            sanitized.to = rest
            // output is intentionally discarded
            void output
        } else if (sanitized.to.type === 'format') {
            // Ensure output mode exists
            sanitized.to.output ??= { mode: 'api' }
            // Ensure format exists
            sanitized.to.format ??= { format_type: 'json', options: {} }
        }
    }

    // Sanitize 'from' definition
    if (sanitized.from) {
        if (sanitized.from.type === 'format') {
            // Ensure source and format exist
            sanitized.from.source ??= { source_type: 'uri', config: {} }
            sanitized.from.format ??= { format_type: 'csv', options: {} }
            // Remove endpoint field from api source type (from.api accepts POST, no endpoint needed)
            if (sanitized.from.source.source_type === 'api' && sanitized.from.source.config) {
                if (sanitized.from.source.config.endpoint !== undefined) {
                    delete sanitized.from.source.config.endpoint
                }
            }
        }
    }

    return sanitized as DslStep
}

/**
 * Sanitizes an array of DSL steps
 */
export function sanitizeDslSteps(steps: unknown[]): DslStep[] {
    if (!Array.isArray(steps)) {
        return []
    }
    return steps.map(sanitizeDslStep)
}

/**
 * Default CSV options
 */
export function defaultCsvOptions(): CsvOptions {
    return { has_header: true, delimiter: ',', escape: undefined, quote: undefined }
}

/**
 * Default step (using new format-based structure)
 */
export function defaultStep(): DslStep {
    return {
        from: {
            type: 'format',
            source: {
                source_type: 'uri',
                config: { uri: '' },
                auth: { type: 'none' },
            },
            format: {
                format_type: 'csv',
                options: defaultCsvOptions(),
            },
            mapping: {},
        },
        transform: { type: 'none' },
        to: {
            type: 'format',
            output: { mode: 'api' },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        },
    }
}

/**
 * Ensures CSV options exist on a step
 */
export function ensureCsvOptions(step: DslStep) {
    if (step.from?.type === 'format' && step.from.format.format_type === 'csv') {
        step.from.format.options ??= defaultCsvOptions()
    }
    if (step.to?.type === 'format' && step.to.format.format_type === 'csv') {
        step.to.format.options ??= defaultCsvOptions()
    }
}

/**
 * Ensures entity filter exists on a step
 */
export function ensureEntityFilter(step: DslStep) {
    if (step.from?.type === 'entity') {
        const f = step.from as {
            filter?: { field: string; operator?: string; value: string }
            mapping?: Record<string, string>
        }
        f.mapping ??= {}
    }
}

/**
 * Converts mapping object to pairs array
 */
export function getMappingPairs(mapping: Mapping): Array<{ k: string; v: string }> {
    const entries = Object.entries(mapping)
    return entries.map(([k, v]) => ({ k, v }))
}

/**
 * Converts pairs array to mapping object
 * Includes pairs where at least one field is non-empty (to preserve empty pairs being edited)
 */
export function pairsToMapping(pairs: Array<{ k: string; v: string }>): Mapping {
    const out: Mapping = {}
    for (const p of pairs) {
        // Include pair if at least one field is non-empty
        // Also include completely empty pairs (for new rows being edited) - use empty string as key
        if (p.k || p.v || (!p.k && !p.v)) {
            // Use empty string as key if k is empty (to preserve empty key entries for new rows)
            const key = p.k || ''
            out[key] = p.v || ''
        }
    }
    return out
}
