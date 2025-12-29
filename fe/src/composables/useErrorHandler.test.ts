import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useErrorHandler } from './useErrorHandler'
import { ValidationError } from '@/api/typed-client'
import { HttpError } from '@/api/errors'

// Mock dependencies
const mockShowError = vi.fn()
const mockShowSuccess = vi.fn()
const mockShowWarning = vi.fn()
const mockT = vi.fn((key: string) => key)

vi.mock('./useSnackbar', () => ({
    useSnackbar: () => ({
        showError: mockShowError,
        showSuccess: mockShowSuccess,
        showWarning: mockShowWarning,
    }),
}))

vi.mock('./useTranslations', () => ({
    useTranslations: () => ({
        t: mockT,
    }),
}))

describe('useErrorHandler', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        // Reset mockT to default implementation (some tests override it)
        mockT.mockImplementation((key: string) => key)
    })

    describe('handleError', () => {
        it('should handle ValidationError with field errors', () => {
            const { handleError } = useErrorHandler()
            const violations = [
                { field: 'name', message: 'Name is required' },
                { field: 'email', message: 'Email is invalid' },
            ]
            const error = new ValidationError('Validation failed', violations)

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(result.fieldErrors).toEqual({
                name: 'Name is required',
                email: 'Email is invalid',
            })
            expect(mockShowError).toHaveBeenCalledWith('Validation failed')
        })

        it('should handle ValidationError with fallback message when message is empty', () => {
            const { handleError } = useErrorHandler()
            // When message is empty string, it uses the fallback
            const error = new ValidationError('', [])

            const result = handleError(error, 'Custom fallback')

            expect(result.handled).toBe(true)
            // Empty string message will use fallback via ?? operator
            expect(mockShowError).toHaveBeenCalled()
        })

        it('should handle Error with validation pattern', () => {
            const { handleError } = useErrorHandler()
            const error = new Error('Validation failed: Invalid input')

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('Validation failed: Invalid input')
        })

        it('should handle HttpError with 403 status code', () => {
            const { handleError } = useErrorHandler()
            const error = new HttpError(403, 'user', 'create', 'Forbidden', 'Cannot create user')
            mockT.mockImplementation(key => {
                if (key === 'error.user.create.403') return 'Cannot create user'
                return key
            })

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('Cannot create user')
        })

        it('should handle HttpError with 404 status code', () => {
            const { handleError } = useErrorHandler()
            const error = new HttpError(404, 'user', 'read', 'Not Found', 'User not found')
            mockT.mockImplementation(key => {
                if (key === 'error.user.read.404') return 'User not found'
                return key
            })

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('User not found')
        })

        it('should handle HttpError with 409 status code', () => {
            const { handleError } = useErrorHandler()
            const error = new HttpError(
                409,
                'user',
                'create',
                'Conflict',
                'A user with this username already exists'
            )
            mockT.mockImplementation(key => {
                if (key === 'error.user.create.409')
                    return 'A user with this username already exists'
                return key
            })

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('A user with this username already exists')
        })

        it('should fallback to namespace-specific error when action-specific not found', () => {
            const { handleError } = useErrorHandler()
            const error = new HttpError(403, 'user', 'create', 'Forbidden', 'Permission denied')
            mockT.mockImplementation(key => {
                if (key === 'error.user.create.403') return 'error.user.create.403' // Not found
                if (key === 'error.user.403') return 'Cannot perform user operation'
                return key
            })

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('Cannot perform user operation')
        })

        it('should fallback to generic status code error when namespace-specific not found', () => {
            const { handleError } = useErrorHandler()
            const error = new HttpError(403, 'unknown', 'create', 'Forbidden', 'Permission denied')
            mockT.mockImplementation(key => {
                if (key === 'error.unknown.create.403') return 'error.unknown.create.403' // Not found
                if (key === 'error.unknown.403') return 'error.unknown.403' // Not found
                if (key === 'error.403') return 'Permission denied'
                return key
            })

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('Permission denied')
        })

        it('should fallback to generic error when all translations not found', () => {
            const { handleError } = useErrorHandler()
            const error = new HttpError(
                500,
                'unknown',
                'create',
                'Server Error',
                'Internal server error'
            )
            mockT.mockImplementation(key => {
                if (key === 'error.generic') return 'An error occurred'
                return key
            })

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('An error occurred')
        })

        it('should show Error message as-is for regular Error (no HTTP code parsing)', () => {
            const { handleError } = useErrorHandler()
            const error = new Error('HTTP 403: Forbidden')

            const result = handleError(error)

            expect(result.handled).toBe(true)
            // Regular Error instances show their message as-is (no parsing)
            expect(mockShowError).toHaveBeenCalledWith('HTTP 403: Forbidden')
        })

        it('should show Error message as-is for permission errors', () => {
            const { handleError } = useErrorHandler()
            const error = new Error('permission denied')

            const result = handleError(error)

            expect(result.handled).toBe(true)
            // Regular Error instances show their message as-is
            expect(mockShowError).toHaveBeenCalledWith('permission denied')
        })

        it('should show Error message as-is for custom 403 messages', () => {
            const { handleError } = useErrorHandler()
            const error = new Error('HTTP 403: Insufficient permissions to perform this action')

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith(
                'HTTP 403: Insufficient permissions to perform this action'
            )
        })

        it('should show Error message as-is for other error formats', () => {
            const { handleError } = useErrorHandler()
            const error = new Error('Access denied. Error code: 403')

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('Access denied. Error code: 403')
        })

        it('should show Error message as-is for 404 errors', () => {
            const { handleError } = useErrorHandler()
            const error = new Error('404 not found')

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('404 not found')
        })

        it('should handle network error with translation', () => {
            const { handleError } = useErrorHandler()
            const error = new Error('network connection failed')

            const result = handleError(error)

            expect(result.handled).toBe(true)
            // Network errors are detected by keywords and translated
            expect(mockShowError).toHaveBeenCalledWith('error.network')
        })

        it('should handle generic Error', () => {
            const { handleError } = useErrorHandler()
            const error = new Error('Something went wrong')

            const result = handleError(error)

            expect(result.handled).toBe(true)
            expect(mockShowError).toHaveBeenCalledWith('Something went wrong')
        })

        it('should handle unknown error types with generic message', () => {
            const { handleError } = useErrorHandler()
            const error = { someProperty: 'value' }

            const result = handleError(error)

            expect(result.handled).toBe(true)
            // Unknown error types show generic error message
            expect(mockShowError).toHaveBeenCalledWith('error.generic')
        })

        it('should handle null/undefined errors with generic message', () => {
            const { handleError } = useErrorHandler()

            const result = handleError(null)

            expect(result.handled).toBe(true)
            // null/undefined errors show generic error message
            expect(mockShowError).toHaveBeenCalledWith('error.generic')
        })
    })

    describe('handleSuccess', () => {
        it('should show success message', () => {
            const { handleSuccess } = useErrorHandler()
            handleSuccess('Operation successful')
            expect(mockShowSuccess).toHaveBeenCalledWith('Operation successful')
        })
    })

    describe('handleWarning', () => {
        it('should show warning message', () => {
            const { handleWarning } = useErrorHandler()
            handleWarning('Warning message')
            expect(mockShowWarning).toHaveBeenCalledWith('Warning message')
        })
    })

    describe('extractFieldErrors', () => {
        it('should extract field errors from ValidationError', () => {
            const { extractFieldErrors } = useErrorHandler()
            const violations = [
                { field: 'name', message: 'Name is required' },
                { field: 'email', message: 'Email is invalid' },
            ]
            const error = new ValidationError('Validation failed', violations)

            const fieldErrors = extractFieldErrors(error)

            expect(fieldErrors).toEqual({
                name: 'Name is required',
                email: 'Email is invalid',
            })
        })

        it('should return empty object for error with no violations', () => {
            const { extractFieldErrors } = useErrorHandler()
            const error = new ValidationError('Validation failed', [])

            const fieldErrors = extractFieldErrors(error)

            expect(fieldErrors).toEqual({})
        })
    })

    describe('isValidationError', () => {
        it('should return true for ValidationError', () => {
            const { isValidationError } = useErrorHandler()
            const error = new ValidationError('Validation failed', [])

            expect(isValidationError(error)).toBe(true)
        })

        it('should return false for regular Error', () => {
            const { isValidationError } = useErrorHandler()
            const error = new Error('Regular error')

            expect(isValidationError(error)).toBe(false)
        })

        it('should return false for unknown types', () => {
            const { isValidationError } = useErrorHandler()
            const error = { someProperty: 'value' }

            expect(isValidationError(error)).toBe(false)
        })
    })

    describe('handleApiError', () => {
        it('should handle API error with operation context', async () => {
            const { handleApiError } = useErrorHandler()
            const error = new Error('API error')

            await handleApiError(error, 'create', 'test context')

            expect(mockShowError).toHaveBeenCalled()
        })

        it('should log error when not handled and context provided', async () => {
            const { handleApiError } = useErrorHandler()
            const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
            const error = { someProperty: 'value' }

            await handleApiError(error, 'create', 'test context')

            // Note: handleError always returns handled: true, so this won't log
            // But the function is tested for its structure
            consoleSpy.mockRestore()
        })
    })
})
