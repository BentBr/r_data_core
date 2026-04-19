import { describe, it, expect } from 'vitest'
import { useDialog } from './useDialog'

describe('useDialog', () => {
    it('should initialize with closed dialog', () => {
        const { showDialog } = useDialog()
        expect(showDialog.value).toBe(false)
    })

    it('should initialize with default config', () => {
        const { dialogConfig } = useDialog()
        expect(dialogConfig.value).toEqual({
            title: '',
            maxWidth: '600px',
            persistent: false,
        })
    })

    it('should initialize with loading false', () => {
        const { loading } = useDialog()
        expect(loading.value).toBe(false)
    })

    it('should initialize with disabled false', () => {
        const { disabled } = useDialog()
        expect(disabled.value).toBe(false)
    })

    it('should open dialog with provided config', () => {
        const { showDialog, dialogConfig, openDialog } = useDialog()
        const config = {
            title: 'Test Dialog',
            maxWidth: '800px',
            persistent: true,
        }
        openDialog(config)
        expect(showDialog.value).toBe(true)
        expect(dialogConfig.value).toEqual(config)
    })

    it('should close dialog and reset state', () => {
        const { showDialog, loading, disabled, openDialog, closeDialog } = useDialog()
        openDialog({ title: 'Test', maxWidth: '600px', persistent: false })
        loading.value = true
        disabled.value = true
        closeDialog()
        expect(showDialog.value).toBe(false)
        expect(loading.value).toBe(false)
        expect(disabled.value).toBe(false)
    })

    it('should set loading state', () => {
        const { loading, setLoading } = useDialog()
        setLoading(true)
        expect(loading.value).toBe(true)
        setLoading(false)
        expect(loading.value).toBe(false)
    })

    it('should set disabled state', () => {
        const { disabled, setDisabled } = useDialog()
        setDisabled(true)
        expect(disabled.value).toBe(true)
        setDisabled(false)
        expect(disabled.value).toBe(false)
    })

    it('should update dialog config when opening new dialog', () => {
        const { dialogConfig, openDialog } = useDialog()
        openDialog({ title: 'First', maxWidth: '600px', persistent: false })
        expect(dialogConfig.value.title).toBe('First')
        openDialog({ title: 'Second', maxWidth: '800px', persistent: true })
        expect(dialogConfig.value.title).toBe('Second')
        expect(dialogConfig.value.maxWidth).toBe('800px')
        expect(dialogConfig.value.persistent).toBe(true)
    })
})
