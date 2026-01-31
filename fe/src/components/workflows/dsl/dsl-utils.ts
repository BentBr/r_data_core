/**
 * Utility functions for DSL configuration
 */

// Import and re-export types from schema for type consistency across the app
import type {
    DslStep,
    FromDef,
    ToDef,
    Transform,
    OutputMode,
    HttpMethod,
    Operand,
    StringOperand,
    AuthConfig,
    SourceConfig,
    FormatConfig,
    DestinationConfig,
} from '@/types/schemas/dsl'

// Re-export all schema types
export type {
    DslStep,
    FromDef,
    ToDef,
    Transform,
    OutputMode,
    HttpMethod,
    Operand,
    StringOperand,
    AuthConfig,
    SourceConfig,
    FormatConfig,
    DestinationConfig,
}

// Local convenience type aliases
export type Mapping = Record<string, string>

export type CsvOptions = {
    has_header?: boolean
    delimiter?: string
    escape?: string
    quote?: string
}

/**
 * Sanitizes a DSL step by removing invalid fields
 * - Removes 'output' field from entity type 'to' definitions
 * - Ensures required fields exist
 */
export function sanitizeDslStep(step: unknown): DslStep {
    const stepObj = step as Record<string, unknown>
    const sanitized: Record<string, unknown> = { ...stepObj }

    // Sanitize 'to' definition
    if (sanitized.to) {
        const toDef = sanitized.to as Record<string, unknown>
        if (toDef.type === 'entity') {
            // Remove 'output' field from entity type
            const { output, ...rest } = toDef
            sanitized.to = rest
            // output is intentionally discarded
            void output
        } else if (toDef.type === 'format') {
            // Ensure output mode exists
            toDef.output ??= { mode: 'api' }
            // Ensure format exists
            toDef.format ??= { format_type: 'json', options: {} }
        } else if (toDef.type === 'next_step') {
            // Ensure mapping exists
            toDef.mapping ??= {}
        }
    }

    // Sanitize 'from' definition
    if (sanitized.from) {
        const fromDef = sanitized.from as Record<string, unknown>
        if (fromDef.type === 'format') {
            // Ensure source and format exist
            fromDef.source ??= { source_type: 'uri', config: {} }
            fromDef.format ??= { format_type: 'csv', options: {} }
            // Remove endpoint field from api source type
            // from.api accepts POST - no endpoint needed
            const source = fromDef.source as Record<string, unknown> | undefined
            if (source?.source_type === 'api' && source.config) {
                const config = source.config as Record<string, unknown>
                if (config.endpoint !== undefined) {
                    delete config.endpoint
                }
            }
            // Note: trigger is now a separate type, not a source type
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
    if (step.from.type === 'format' && step.from.format.format_type === 'csv') {
        step.from.format.options ??= defaultCsvOptions()
    }
    if (step.to.type === 'format' && step.to.format.format_type === 'csv') {
        step.to.format.options ??= defaultCsvOptions()
    }
}

/**
 * Ensures entity filter exists on a step
 */
export function ensureEntityFilter(step: DslStep) {
    if (step.from.type === 'entity') {
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
