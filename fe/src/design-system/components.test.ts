import { describe, it, expect } from 'vitest'
import { getDialogMaxWidth, dialogConfigs, type DialogSize } from './components'
import { dialogTokens } from './tokens'

describe('getDialogMaxWidth', () => {
    it('returns the default size when called with no argument', () => {
        expect(getDialogMaxWidth()).toBe(dialogTokens.maxWidths.default)
    })

    it('returns each defined size', () => {
        const sizes = Object.keys(dialogTokens.maxWidths) as DialogSize[]

        expect(sizes.length).toBeGreaterThan(0)
        for (const size of sizes) {
            expect(getDialogMaxWidth(size)).toBe(dialogTokens.maxWidths[size])
        }
    })

    // Written against the tokens rather than literals so adding or retuning a
    // size does not require editing this file — the previous hand-maintained
    // union is exactly what let a component hardcode a width instead.
    it('exposes every token through the accessor', () => {
        expect(Object.keys(dialogConfigs.maxWidths).sort()).toEqual(
            Object.keys(dialogTokens.maxWidths).sort()
        )
    })

    it('gives every size a CSS pixel length', () => {
        for (const value of Object.values(dialogTokens.maxWidths)) {
            expect(value).toMatch(/^\d+px$/)
        }
    })

    it('orders the sizes from narrowest to widest', () => {
        // small < default < form < editor < wide. A size that breaks the
        // ordering is almost always a typo, and nothing else would catch it.
        const order: DialogSize[] = ['small', 'default', 'form', 'editor', 'wide']
        const widths = order.map(s => parseInt(getDialogMaxWidth(s), 10))

        expect(widths).toEqual([...widths].sort((a, b) => a - b))
        expect(new Set(widths).size).toBe(widths.length)
    })

    it('covers every token in the ordering check', () => {
        // Guards the test above: a new token must be placed in that list
        // rather than silently skipped.
        const order: DialogSize[] = ['small', 'default', 'form', 'editor', 'wide']

        expect(order.sort()).toEqual(Object.keys(dialogTokens.maxWidths).sort())
    })

    it('offers the editor size the email template editor needs', () => {
        expect(getDialogMaxWidth('editor')).toBe('900px')
    })
})
