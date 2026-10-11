import { describe, it, expect } from 'vitest'
import { DIALOG_SIZES, dialogTokens } from './tokens'
import { DialogConfigSchema } from '@/types/schemas'

/**
 * Structural guard, in the spirit of the backend's tests/architecture.rs.
 *
 * Dialog widths drifted out of the design system in two different ways, and
 * neither failed anything: components hardcoded `max-width="900px"` on the
 * element, and pages passed raw `maxWidth: '500px'` through the dialog config
 * — a size that was never a token at all. Unit tests around getDialogMaxWidth
 * cannot see a caller that never calls it, so this reads the sources.
 *
 * `import.meta.glob` rather than node:fs: the app's tsconfig carries no Node
 * types, and Vite resolves this at build time in both environments.
 */
const sources = import.meta.glob('../**/*.{vue,ts}', {
    query: '?raw',
    import: 'default',
    eager: true,
}) as Record<string, string>

const production = Object.entries(sources).filter(([path]) => !path.includes('.test.'))

/** The opening `<v-dialog ...>` tags in a source, so sibling elements with
 *  their own max-width are not mistaken for a dialog's. */
function dialogTags(source: string): string[] {
    return [...source.matchAll(/<v-dialog\b[^>]*>/g)].map(m => m[0])
}

describe('dialog widths come from the design system', () => {
    it('finds sources to check', () => {
        expect(production.length).toBeGreaterThan(50)
    })

    it('no dialog element hardcodes a width', () => {
        // Covers the attribute and the bound-number forms, on one line or
        // many, and only inside the dialog's own tag.
        const offenders = production
            .filter(([, src]) =>
                dialogTags(src).some(tag => /:?max-width=["'](\d+(px)?)["']/.test(tag))
            )
            .map(([path]) => path)

        expect(offenders).toEqual([])
    })

    it('no dialog config supplies a raw CSS length', () => {
        const offenders = production
            .filter(([, src]) => /maxWidth:\s*['"][^'"]*\d\s*(px|rem|em|%|vw)['"]/.test(src))
            .map(([path]) => path)

        expect(offenders).toEqual([])
    })

    it('the config schema accepts only the sizes the tokens define', () => {
        // Cross-module parity: the zod enum and the token record are separate
        // declarations, and TypeScript alone will not notice them diverging.
        for (const size of DIALOG_SIZES) {
            expect(DialogConfigSchema.safeParse({ title: 't', maxWidth: size }).success).toBe(true)
        }
        expect(DialogConfigSchema.safeParse({ title: 't', maxWidth: '500px' }).success).toBe(false)
        expect(Object.keys(dialogTokens.maxWidths).sort()).toEqual([...DIALOG_SIZES].sort())
    })
})
