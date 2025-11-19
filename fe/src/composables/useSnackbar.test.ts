import { describe, it, expect } from 'vitest'
import { useSnackbar } from './useSnackbar'

describe('useSnackbar', () => {
    it('should initialize with null snackbar', () => {
        const { currentSnackbar } = useSnackbar()
        expect(currentSnackbar.value).toBeNull()
    })

    it('should show success message with default timeout', () => {
        const { currentSnackbar, showSuccess } = useSnackbar()
        showSuccess('Operation successful')
        expect(currentSnackbar.value).toEqual({
            message: 'Operation successful',
            color: 'success',
            timeout: 3000,
            persistent: false,
        })
    })

    it('should show success message with custom timeout', () => {
        const { currentSnackbar, showSuccess } = useSnackbar()
        showSuccess('Operation successful', 5000)
        expect(currentSnackbar.value?.timeout).toBe(5000)
    })

    it('should show error message with default timeout', () => {
        const { currentSnackbar, showError } = useSnackbar()
        showError('An error occurred')
        expect(currentSnackbar.value).toEqual({
            message: 'An error occurred',
            color: 'error',
            timeout: 5000,
            persistent: false,
        })
    })

    it('should show error message with custom timeout', () => {
        const { currentSnackbar, showError } = useSnackbar()
        showError('An error occurred', 10000)
        expect(currentSnackbar.value?.timeout).toBe(10000)
    })

    it('should show warning message with default timeout', () => {
        const { currentSnackbar, showWarning } = useSnackbar()
        showWarning('Warning message')
        expect(currentSnackbar.value).toEqual({
            message: 'Warning message',
            color: 'warning',
            timeout: 4000,
            persistent: false,
        })
    })

    it('should show warning message with custom timeout', () => {
        const { currentSnackbar, showWarning } = useSnackbar()
        showWarning('Warning message', 6000)
        expect(currentSnackbar.value?.timeout).toBe(6000)
    })

    it('should show info message with default timeout', () => {
        const { currentSnackbar, showInfo } = useSnackbar()
        showInfo('Info message')
        expect(currentSnackbar.value).toEqual({
            message: 'Info message',
            color: 'info',
            timeout: 3000,
            persistent: false,
        })
    })

    it('should show info message with custom timeout', () => {
        const { currentSnackbar, showInfo } = useSnackbar()
        showInfo('Info message', 7000)
        expect(currentSnackbar.value?.timeout).toBe(7000)
    })

    it('should clear snackbar', () => {
        const { currentSnackbar, showSuccess, clearSnackbar } = useSnackbar()
        showSuccess('Test message')
        expect(currentSnackbar.value).not.toBeNull()
        clearSnackbar()
        expect(currentSnackbar.value).toBeNull()
    })

    it('should replace previous snackbar when showing new one', () => {
        const { currentSnackbar, showSuccess, showError } = useSnackbar()
        showSuccess('First message')
        expect(currentSnackbar.value?.message).toBe('First message')
        showError('Second message')
        expect(currentSnackbar.value?.message).toBe('Second message')
        expect(currentSnackbar.value?.color).toBe('error')
    })
})
