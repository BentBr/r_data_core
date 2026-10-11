import { describe, it, expect } from 'vitest'

/**
 * Structural guard, alongside dialog-width.guard.test.ts.
 *
 * Dialog open/closed state was bound three different ways across fifteen
 * components: `:model-value` with a manual `@update:model-value="$emit(...)"`,
 * a local `computed` get/set wrapper under three different names
 * (`showDialog`, `dialogVisible`), and the prop read directly. All three
 * behave the same, which is why the drift went unnoticed — and why a reader
 * had to work out which convention each file used.
 *
 * They are now all `defineModel<boolean>()`. This keeps them that way.
 */
const sources = import.meta.glob('../**/*.vue', {
    query: '?raw',
    import: 'default',
    eager: true,
}) as Record<string, string>

const dialogs = Object.entries(sources).filter(([, src]) => src.includes('<v-dialog'))

describe('dialog v-model binding is consistent', () => {
    it('finds the dialog components', () => {
        expect(dialogs.length).toBeGreaterThan(10)
    })

    it('no dialog re-emits update:modelValue by hand', () => {
        // defineModel does this. A manual re-emit means the component also
        // declares the prop and the event, which is the shape being removed.
        const offenders = dialogs
            .filter(([, src]) => /@update:model-value="\$emit\('update:modelValue'/.test(src))
            .map(([path]) => path)

        expect(offenders).toEqual([])
    })

    it('no dialog declares a modelValue prop', () => {
        const offenders = dialogs
            .filter(([, src]) => /\n\s*modelValue\??: boolean/.test(src))
            .map(([path]) => path)

        expect(offenders).toEqual([])
    })

    it('no dialog wraps the model in a hand-written computed', () => {
        // `const showDialog = computed({ get: () => props.modelValue, ... })`
        const offenders = dialogs
            .filter(([, src]) => /computed\(\{[\s\S]{0,200}?props\.modelValue/.test(src))
            .map(([path]) => path)

        expect(offenders).toEqual([])
    })

    it('every dialog component declares a model', () => {
        // Excludes pages, which host dialogs rather than being one.
        const components = dialogs.filter(([path]) => path.includes('/components/'))
        const missing = components
            .filter(([, src]) => !src.includes('defineModel'))
            .map(([path]) => path)

        expect(missing).toEqual([])
    })
})
