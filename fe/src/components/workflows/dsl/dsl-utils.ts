/**
 * Utility functions for DSL configuration
 */

export type Mapping = Record<string, string>

export type CsvOptions = { header?: boolean; delimiter?: string; escape?: string; quote?: string }

export type FromDef =
    | { type: 'csv'; uri: string; options: CsvOptions; mapping: Mapping }
    | { type: 'json'; uri: string; mapping: Mapping }
    | { type: 'entity'; entity_definition: string; filter: { field: string; value: string }; mapping: Mapping }

export type OperandField = { kind: 'field'; field: string }
export type OperandConst = { kind: 'const'; value: number }
export type Operand = OperandField | OperandConst

export type StringOperandField = { kind: 'field'; field: string }
export type StringOperandConst = { kind: 'const_string'; value: string }
export type StringOperand = StringOperandField | StringOperandConst

export type Transform =
    | { type: 'none' }
    | { type: 'arithmetic'; target: string; left: Operand; op: 'add' | 'sub' | 'mul' | 'div'; right: Operand }
    | { type: 'concat'; target: string; left: StringOperand; separator?: string; right: StringOperand }

export type ToDef =
    | { type: 'csv'; output: 'api' | 'download'; options: CsvOptions; mapping: Mapping }
    | { type: 'json'; output: 'api' | 'download'; mapping: Mapping }
    | { type: 'entity'; entity_definition: string; path: string; mode: 'create' | 'update'; update_key?: string; identify?: { field: string; value: string }; mapping: Mapping }

export type DslStep = { from: FromDef; transform: Transform; to: ToDef }

/**
 * Sanitizes a DSL step by removing invalid fields
 * - Removes 'output' field from entity type 'to' definitions
 * - Ensures required fields exist
 */
export function sanitizeDslStep(step: any): DslStep {
    const sanitized: any = { ...step }

    // Sanitize 'to' definition
    if (sanitized.to) {
        if (sanitized.to.type === 'entity') {
            // Remove 'output' field from entity type (it should only exist for csv/json)
            const { output, ...rest } = sanitized.to
            sanitized.to = rest
        } else if (sanitized.to.type === 'csv' || sanitized.to.type === 'json') {
            // Ensure 'output' field exists for csv/json types
            if (!sanitized.to.output) {
                sanitized.to.output = 'api'
            }
        }
    }

    return sanitized as DslStep
}

/**
 * Sanitizes an array of DSL steps
 */
export function sanitizeDslSteps(steps: any[]): DslStep[] {
    if (!Array.isArray(steps)) {
        return []
    }
    return steps.map(sanitizeDslStep)
}

/**
 * Default CSV options
 */
export function defaultCsvOptions(): CsvOptions {
    return { header: true, delimiter: ',', escape: undefined, quote: undefined }
}

/**
 * Default step
 */
export function defaultStep(): DslStep {
    return {
        from: { type: 'csv', uri: '', options: defaultCsvOptions(), mapping: {} },
        transform: { type: 'none' },
        to: { type: 'json', output: 'api', mapping: {} },
    }
}

/**
 * Ensures CSV options exist on a step
 */
export function ensureCsvOptions(step: DslStep) {
    if (step.from?.type === 'csv') {
        const f: any = step.from
        if (!f.options) {
            f.options = defaultCsvOptions()
        }
    }
    if (step.to?.type === 'csv') {
        const t: any = step.to
        if (!t.options) {
            t.options = defaultCsvOptions()
        }
    }
}

/**
 * Ensures entity filter exists on a step
 */
export function ensureEntityFilter(step: DslStep) {
    if (step.from?.type === 'entity') {
        const f: any = step.from
        if (!f.filter) {
            f.filter = { field: '', value: '' }
        }
        if (!f.mapping) {
            f.mapping = {}
        }
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

