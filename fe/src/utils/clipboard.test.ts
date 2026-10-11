import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { copyText } from './clipboard'

describe('copyText', () => {
    const originalClipboard = navigator.clipboard

    beforeEach(() => {
        vi.restoreAllMocks()
    })

    afterEach(() => {
        Object.defineProperty(navigator, 'clipboard', {
            value: originalClipboard,
            configurable: true,
            writable: true,
        })
    })

    const setClipboard = (value: unknown) => {
        Object.defineProperty(navigator, 'clipboard', {
            value,
            configurable: true,
            writable: true,
        })
    }

    it('uses the Clipboard API when it is available', async () => {
        const writeText = vi.fn().mockResolvedValue(undefined)
        setClipboard({ writeText })

        await expect(copyText('secret-key')).resolves.toBe(true)
        expect(writeText).toHaveBeenCalledWith('secret-key')
    })

    it('falls back when navigator.clipboard is absent', async () => {
        // The real failure this guards: outside a secure context the whole
        // API is undefined, and reading .writeText off it throws
        // synchronously — so a promise .catch() never runs.
        setClipboard(undefined)
        const exec = vi.fn().mockReturnValue(true)
        document.execCommand = exec as unknown as typeof document.execCommand

        await expect(copyText('secret-key')).resolves.toBe(true)
        expect(exec).toHaveBeenCalledWith('copy')
    })

    it('falls back when the Clipboard API rejects', async () => {
        setClipboard({ writeText: vi.fn().mockRejectedValue(new Error('denied')) })
        const exec = vi.fn().mockReturnValue(true)
        document.execCommand = exec as unknown as typeof document.execCommand

        await expect(copyText('secret-key')).resolves.toBe(true)
        expect(exec).toHaveBeenCalledWith('copy')
    })

    it('reports failure when both paths fail', async () => {
        setClipboard(undefined)
        document.execCommand = vi
            .fn()
            .mockReturnValue(false) as unknown as typeof document.execCommand

        await expect(copyText('secret-key')).resolves.toBe(false)
    })

    it('leaves no textarea behind', async () => {
        setClipboard(undefined)
        document.execCommand = vi
            .fn()
            .mockReturnValue(true) as unknown as typeof document.execCommand

        await copyText('secret-key')

        expect(document.querySelectorAll('textarea')).toHaveLength(0)
    })

    it('removes the textarea even when execCommand throws', async () => {
        setClipboard(undefined)
        document.execCommand = vi.fn(() => {
            throw new Error('boom')
        }) as unknown as typeof document.execCommand

        await expect(copyText('secret-key')).resolves.toBe(false)
        expect(document.querySelectorAll('textarea')).toHaveLength(0)
    })
})
