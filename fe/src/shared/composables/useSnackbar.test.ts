import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { useSnackbar } from './useSnackbar'

describe('useSnackbar', () => {
    beforeEach(() => {
        // Clear snackbar state before each test
        const { clearSnackbar } = useSnackbar()
        clearSnackbar()
        vi.useFakeTimers()
    })

    afterEach(() => {
        vi.useRealTimers()
    })

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

    describe('singleton pattern', () => {
        it('should share state between multiple useSnackbar instances', () => {
            const instance1 = useSnackbar()
            const instance2 = useSnackbar()

            // Show message from instance 1
            instance1.showSuccess('Shared message')

            // Should be visible from instance 2
            expect(instance2.currentSnackbar.value?.message).toBe('Shared message')
        })

        it('should clear state from any instance', () => {
            const instance1 = useSnackbar()
            const instance2 = useSnackbar()

            instance1.showError('Error message')
            expect(instance2.currentSnackbar.value).not.toBeNull()

            instance2.clearSnackbar()
            expect(instance1.currentSnackbar.value).toBeNull()
        })
    })

    describe('auto-clear timeout', () => {
        it('should auto-clear snackbar after timeout plus buffer', () => {
            const { currentSnackbar, showSuccess } = useSnackbar()

            showSuccess('Test message', 3000)
            expect(currentSnackbar.value).not.toBeNull()

            // Advance time past the timeout + 500ms buffer
            vi.advanceTimersByTime(3500)

            expect(currentSnackbar.value).toBeNull()
        })

        it('should cancel previous auto-clear when showing new snackbar', () => {
            const { currentSnackbar, showSuccess, showError } = useSnackbar()

            showSuccess('First message', 2000)

            // Advance time partially
            vi.advanceTimersByTime(1000)
            expect(currentSnackbar.value?.message).toBe('First message')

            // Show new snackbar (should cancel previous timeout)
            showError('Second message', 3000)
            expect(currentSnackbar.value?.message).toBe('Second message')

            // Advance past original timeout - should still show second message
            vi.advanceTimersByTime(1600) // 1000 + 1600 = 2600ms (past first timeout + buffer)
            expect(currentSnackbar.value?.message).toBe('Second message')

            // Advance past second timeout + buffer
            vi.advanceTimersByTime(2000)
            expect(currentSnackbar.value).toBeNull()
        })

        it('should clear timeout when manually clearing snackbar', () => {
            const { currentSnackbar, showSuccess, clearSnackbar } = useSnackbar()

            showSuccess('Test message', 5000)
            clearSnackbar()

            expect(currentSnackbar.value).toBeNull()

            // Advance past the original timeout - should stay null
            vi.advanceTimersByTime(6000)
            expect(currentSnackbar.value).toBeNull()
        })
    })
})
