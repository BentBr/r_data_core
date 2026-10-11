import { describe, it, expect, afterEach } from 'vitest'
import { mount, DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ApiKeyViewDialog from './ApiKeyViewDialog.vue'
import type { ApiKey } from '@/types/schemas'

const apiKey = {
    uuid: 'key-1',
    name: 'ci-deploy',
    description: 'used by CI',
    is_active: true,
    created_at: '2026-01-01T00:00:00Z',
    expires_at: null,
    last_used_at: null,
} as unknown as ApiKey

describe('ApiKeyViewDialog', () => {
    let wrapper: VueWrapper | undefined

    afterEach(() => {
        wrapper?.unmount()
    })

    const body = () => new DOMWrapper(document.body)

    const mountDialog = (props: Record<string, unknown> = {}) =>
        mount(ApiKeyViewDialog, {
            props: { modelValue: true, apiKey, ...props },
            global: { stubs: { teleport: false } },
        })

    // This component delegates its dialog, card, title and padding to
    // DialogManager and supplies only a body and one button. These assert the
    // shell actually renders them, which is what makes the shared layout
    // worth having rather than an unused API.
    it('renders its title through the shared shell', () => {
        wrapper = mountDialog()

        expect(body().text()).toContain('title')
    })

    it('renders the key details in the shell body', () => {
        wrapper = mountDialog()

        expect(body().text()).toContain('ci-deploy')
        expect(body().text()).toContain('used by CI')
    })

    it('replaces the shell buttons with its own close action', () => {
        wrapper = mountDialog()
        const labels = body()
            .findAll('button')
            .map(b => b.text())

        expect(labels.some(l => l.includes('close'))).toBe(true)
        // The shell's own confirm/cancel pair must not appear alongside it.
        expect(labels.some(l => l.includes('confirm'))).toBe(false)
    })

    it('closes through the scoped close the shell provides', async () => {
        wrapper = mountDialog()

        const close = body()
            .findAll('button')
            .filter(b => b.text().includes('close'))[0] as DOMWrapper<Element> | undefined
        await close?.trigger('click')

        const emits = wrapper.emitted('update:modelValue') ?? []
        expect(emits[emits.length - 1]).toEqual([false])
    })

    it('renders nothing in the body when there is no key', () => {
        wrapper = mountDialog({ apiKey: null })

        expect(body().text()).not.toContain('ci-deploy')
    })
})
