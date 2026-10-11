import { describe, it, expect, vi, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import ApiKeyCreatedDialog from './ApiKeyCreatedDialog.vue'

const KEY = 'mIEnl2astkJXEUMsdNg+MY8Er/ububNoAjJgMIoDqHI'

describe('ApiKeyCreatedDialog', () => {
    let wrapper: VueWrapper | undefined
    const originalClipboard = navigator.clipboard

    const setClipboard = (value: unknown) => {
        Object.defineProperty(navigator, 'clipboard', {
            value,
            configurable: true,
            writable: true,
        })
    }

    afterEach(() => {
        wrapper?.unmount()
        setClipboard(originalClipboard)
        vi.restoreAllMocks()
    })

    const mountDialog = () =>
        mount(ApiKeyCreatedDialog, {
            props: { modelValue: true, apiKey: KEY },
            global: { stubs: { teleport: false } },
        })

    it('copies through the Clipboard API when it is available', async () => {
        const writeText = vi.fn().mockResolvedValue(undefined)
        setClipboard({ writeText })
        wrapper = mountDialog()

        await (wrapper.vm as unknown as { copyApiKey: () => Promise<void> }).copyApiKey()

        expect(writeText).toHaveBeenCalledWith(KEY)
        expect(wrapper.emitted('copy-success')).toBeTruthy()
    })

    // The reported bug. The admin is served over plain HTTP on a named host
    // locally, so navigator.clipboard is undefined. The old code read
    // .writeText off it, which throws synchronously — before any promise
    // exists — so the .catch() never ran and the button silently did nothing.
    it('still copies when navigator.clipboard is undefined', async () => {
        setClipboard(undefined)
        const exec = vi.fn().mockReturnValue(true)
        document.execCommand = exec as unknown as typeof document.execCommand
        wrapper = mountDialog()

        await (wrapper.vm as unknown as { copyApiKey: () => Promise<void> }).copyApiKey()

        expect(exec).toHaveBeenCalledWith('copy')
        expect(wrapper.emitted('copy-success')).toBeTruthy()
        expect(wrapper.emitted('copy-failure')).toBeFalsy()
    })

    it('reports failure rather than failing silently', async () => {
        setClipboard(undefined)
        document.execCommand = vi
            .fn()
            .mockReturnValue(false) as unknown as typeof document.execCommand
        wrapper = mountDialog()

        await (wrapper.vm as unknown as { copyApiKey: () => Promise<void> }).copyApiKey()

        // The key is shown exactly once, so a copy that quietly failed is the
        // worst outcome: the user closes the dialog believing they have it.
        expect(wrapper.emitted('copy-failure')).toBeTruthy()
        expect(wrapper.emitted('copy-success')).toBeFalsy()
    })

    it('shows the key so it can be copied by hand as a last resort', () => {
        setClipboard(undefined)
        wrapper = mountDialog()

        expect(document.body.querySelector('#apiKey')).not.toBeNull()
    })
})
