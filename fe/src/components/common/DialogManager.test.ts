import { describe, it, expect, afterEach } from 'vitest'
import { mount, DOMWrapper, type VueWrapper } from '@vue/test-utils'
import DialogManager from './DialogManager.vue'
import type { DialogConfig } from '@/types/schemas'

describe('DialogManager', () => {
    let wrapper: VueWrapper | undefined

    afterEach(() => {
        wrapper?.unmount()
    })

    const config = (over: Partial<DialogConfig> = {}): DialogConfig =>
        ({ title: 'Revoke key', ...over }) as DialogConfig

    // `teleport: false` is required, not stylistic. v-dialog teleports its
    // card, and the global stub cannot hand the children back in a queryable
    // form — it renders them as the string "[object Object]". Real teleport
    // puts the card on document.body, so assertions go through body() and
    // every mount is unmounted in afterEach or it leaks into the next test.
    const mountDialog = (props: Record<string, unknown> = {}) =>
        mount(DialogManager, {
            props: { modelValue: true, config: config(), ...props },
            slots: { default: '<p class="body-slot">Are you sure?</p>' },
            global: { stubs: { teleport: false } },
        })

    const body = () => new DOMWrapper(document.body)
    // Typed as possibly-absent on purpose: findAll()[0] is typed as always
    // defined, but the whole point of several assertions below is that the
    // button may not be there.
    const buttonWith = (label: string): DOMWrapper<Element> | undefined =>
        body()
            .findAll('button')
            .filter(b => b.text().includes(label))[0] as DOMWrapper<Element> | undefined

    it('renders the configured title and the slot body', () => {
        wrapper = mountDialog()

        expect(body().text()).toContain('Revoke key')
        expect(body().find('.body-slot').exists()).toBe(true)
    })

    // The bug this pins. confirmText/cancelText used to default to the
    // literals 'Confirm' and 'Cancel', and five of the six callers pass
    // neither — so those dialogs rendered English buttons regardless of the
    // selected language.
    it('falls back to translated button labels, not English literals', () => {
        wrapper = mountDialog()
        const text = body().text()

        // The translation layer is stubbed to echo the final key segment, so
        // a literal 'Confirm' here would mean the English default leaked back.
        expect(text).toContain('confirm')
        expect(text).toContain('cancel')
    })

    it('prefers explicit button labels when given', () => {
        wrapper = mountDialog({ confirmText: 'Revoke', cancelText: 'Keep' })
        const text = body().text()

        expect(text).toContain('Revoke')
        expect(text).toContain('Keep')
    })

    it('emits confirm when the confirm button is pressed', async () => {
        wrapper = mountDialog({ confirmText: 'Revoke' })

        await buttonWith('Revoke')?.trigger('click')

        expect(wrapper.emitted('confirm')).toBeTruthy()
    })

    it('closes without confirming when cancelled', async () => {
        wrapper = mountDialog({ cancelText: 'Keep' })

        await buttonWith('Keep')?.trigger('click')

        const emits = wrapper.emitted('update:modelValue') ?? []
        expect(emits[emits.length - 1]).toEqual([false])
        expect(wrapper.emitted('confirm')).toBeFalsy()
    })

    it('hides the confirm button when asked', () => {
        wrapper = mountDialog({ showConfirmButton: false, confirmText: 'Revoke' })

        expect(buttonWith('Revoke')).toBeUndefined()
        // The cancel button must survive — hiding confirm should not leave a
        // dialog with no way out.
        expect(buttonWith('cancel')).toBeDefined()
    })

    it('disables confirm while an action is in flight', () => {
        wrapper = mountDialog({ disabled: true, confirmText: 'Revoke' })

        expect(buttonWith('Revoke')?.attributes('disabled')).toBeDefined()
    })

    it('uses the config max width over the default', () => {
        wrapper = mountDialog({ config: config({ maxWidth: '1234px' }) })

        expect(body().html()).toContain('1234px')
    })

    it('falls back to the design-system width when config gives none', () => {
        wrapper = mountDialog()

        // 600px is the 'default' dialog token; a hardcoded literal here would
        // not survive a token change, which is the point of the helper.
        expect(body().html()).toContain('600px')
    })

    it('is not persistent unless the config says so', () => {
        wrapper = mountDialog()
        expect(wrapper.findComponent({ name: 'VDialog' }).props('persistent')).toBe(false)

        wrapper.unmount()
        wrapper = mountDialog({ config: config({ persistent: true }) })
        expect(wrapper.findComponent({ name: 'VDialog' }).props('persistent')).toBe(true)
    })
})
