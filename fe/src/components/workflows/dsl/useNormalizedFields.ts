import { computed, type Ref, isRef } from 'vue'
import type { FromDef, DslStep } from './dsl-utils'

/**
 * Composable to extract normalized field names from a FromDef or DslStep
 * Normalized fields are the VALUES in the mapping object
 * Example: { "source_price": "price" } -> normalized field is "price"
 *
 * @param input - DslStep, FromDef, or reactive reference to either
 * @returns Object with normalizedFields computed property
 */
export function useNormalizedFields(input: DslStep | FromDef | Ref<DslStep | FromDef | undefined>) {
    // Normalize input to a Ref
    const fromDefRef = computed<FromDef | undefined>(() => {
        if (isRef(input)) {
            const val = input.value
            if (!val) {
                return undefined
            }
            // Check if it's a DslStep (has 'from' property)
            if ('from' in val) {
                return val.from
            }
            // Otherwise it's a FromDef
            return val
        } else {
            // Direct object
            if ('from' in input) {
                return input.from
            }
            return input
        }
    })

    const normalizedFields = computed<string[]>(() => {
        const fromDef = fromDefRef.value
        if (!fromDef) {
            return []
        }

        const mapping = fromDef.mapping

        if (!mapping || typeof mapping !== 'object') {
            return []
        }

        // Normalized fields are the VALUES in the mapping
        // mapping: { "source_field": "normalized_field" }
        const fields = Object.values(mapping).filter(v => typeof v === 'string' && v.trim() !== '')

        // Deduplicate and sort
        return [...new Set(fields)].sort()
    })

    return { normalizedFields }
}
