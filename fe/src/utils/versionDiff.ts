/**
 * Utilities for computing human-friendly diffs between two entity version snapshots.
 *
 * Why:
 * - Version snapshots are stored as arbitrary/nested JSON. For a compact "changes table"
 *   in the UI we need a flat, stable list of "field -> value" pairs that can be compared.
 *
 * What it does:
 * - flattenObject: Recursively flattens a nested object into dot-notated keys
 *   (e.g. { a: { b: 1 } } => { "a.b": "1" }). All values are normalized to strings so
 *   rendering is simple and consistent (including arrays/objects via JSON.stringify).
 * - computeDiffRows: Produces a sorted set of rows for the union of keys in both snapshots.
 *   Each row contains the field path, value in A, value in B and a boolean flag whether it changed.
 *
 * Notes:
 * - This intentionally treats arrays/objects as opaque values (stringified) to keep UI fast
 *   and compact. If deep structural diffs are ever required, this module can be extended
 *   without changing the consuming component API.
 */
export type DiffRow = { field: string; a: string; b: string; changed: boolean }

export function flattenObject(
    obj: Record<string, unknown> | undefined,
    prefix = ''
): Record<string, string> {
    const out: Record<string, string> = {}
    if (!obj) {
        return out
    }
    for (const [k, v] of Object.entries(obj)) {
        const key = prefix ? `${prefix}.${k}` : k
        if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
            Object.assign(out, flattenObject(v as Record<string, unknown>, key))
        } else {
            out[key] = typeof v === 'object' ? JSON.stringify(v) : String(v ?? '')
        }
    }
    return out
}

export function computeDiffRows(a: Record<string, unknown>, b: Record<string, unknown>): DiffRow[] {
    const af = flattenObject(a)
    const bf = flattenObject(b)
    const keys = Array.from(new Set([...Object.keys(af), ...Object.keys(bf)])).sort()
    return keys.map(k => {
        const va = af[k] ?? ''
        const vb = bf[k] ?? ''
        return { field: k, a: va, b: vb, changed: va !== vb }
    })
}
