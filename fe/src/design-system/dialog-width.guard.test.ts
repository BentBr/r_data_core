import { describe, it, expect } from 'vitest'
import { dialogTokens } from './tokens'

/**
 * Structural guard, in the spirit of the backend's tests/architecture.rs.
 *
 * Dialog widths drifted out of the design system twice — SystemLogsViewer
 * pinned 800px and EmailTemplateEditor 900px — and nothing failed, because a
 * hardcoded attribute is perfectly valid Vue. The unit tests around
 * getDialogMaxWidth cannot see a component that never calls it, so this reads
 * the sources instead.
 *
 * `import.meta.glob` rather than node:fs: the app's tsconfig carries no Node
 * types, and Vite resolves this at build time in both the browser and test
 * environments.
 */
const sources = import.meta.glob('../**/*.vue', {
    query: '?raw',
    import: 'default',
    eager: true,
}) as Record<string, string>

describe('dialog widths come from the design system', () => {
    const entries = Object.entries(sources)

    it('finds components to check', () => {
        expect(entries.length).toBeGreaterThan(20)
    })

    it('no component hardcodes a v-dialog max-width', () => {
        // Only the attribute form on a dialog; `style="max-width: 240px"` on a
        // table column is a different thing and stays allowed.
        const offenders = entries
            .filter(([, source]) => source.includes('<v-dialog'))
            .filter(([, source]) => /\n\s*:?max-width="\d+px"/.test(source))
            .map(([path]) => path)

        expect(offenders).toEqual([])
    })

    it('every size a component asks for exists as a token', () => {
        const known = new Set(Object.keys(dialogTokens.maxWidths))
        const unknown: string[] = []

        for (const [path, source] of entries) {
            for (const [, size] of source.matchAll(/getDialogMaxWidth\('([a-z]+)'\)/g)) {
                if (!known.has(size)) {
                    unknown.push(`${path}: '${size}'`)
                }
            }
        }

        expect(unknown).toEqual([])
    })
})
