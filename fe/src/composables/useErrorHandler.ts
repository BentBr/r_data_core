import { useSnackbar } from './useSnackbar'
import { useTranslations } from './useTranslations'
import { ValidationError } from '@/api/typed-client'

export interface FieldError {
    field: string
    message: string
    code?: string
}

export interface ErrorHandlerResult {
    handled: boolean
    fieldErrors?: Record<string, string>
}

export function useErrorHandler() {
    const { showError, showSuccess, showWarning } = useSnackbar()
    const { t } = useTranslations()

    /**
     * Handle errors with proper user feedback
     * Returns field errors if it's a validation error
     */
    const handleError = (error: unknown, fallbackMessage?: string): ErrorHandlerResult => {
        // Handle ValidationError specifically
        if (error instanceof ValidationError) {
            // Convert violations to a field error map
            const fieldErrors: Record<string, string> = {}
            for (const violation of error.violations) {
                fieldErrors[violation.field] = violation.message
            }

            // Show general error message
            showError(
                error.message ?? fallbackMessage ?? t('validation.error') ?? 'Validation failed'
            )

            return {
                handled: true,
                fieldErrors,
            }
        }

        // Handle regular Error instances
        if (error instanceof Error) {
            const errorMessage = error.message

            // Check for specific error patterns
            if (
                errorMessage.includes('Validation') ||
                errorMessage.includes('Validation failed') ||
                errorMessage.includes('Unknown fields')
            ) {
                showError(errorMessage)
                return { handled: true }
            }

            if (errorMessage.includes('409') || errorMessage.includes('conflict')) {
                showError(t('errors.conflict') ?? 'This item already exists')
                return { handled: true }
            }

            if (errorMessage.includes('401') || errorMessage.includes('authentication')) {
                showError(t('errors.authentication') ?? 'Authentication required')
                return { handled: true }
            }

            if (errorMessage.includes('403') || errorMessage.includes('permission')) {
                showError(t('errors.permission') ?? 'Permission denied')
                return { handled: true }
            }

            if (errorMessage.includes('404') || errorMessage.includes('not found')) {
                showError(t('errors.not_found') ?? 'Resource not found')
                return { handled: true }
            }

            if (errorMessage.includes('network') || errorMessage.includes('connection')) {
                showError(t('errors.network') ?? 'Network error. Please try again.')
                return { handled: true }
            }

            // Generic error
            showError(errorMessage ?? fallbackMessage ?? t('errors.generic') ?? 'An error occurred')
            return { handled: true }
        }

        // Handle unknown error types
        const message =
            fallbackMessage ??
            t('errors.generic') ??
            'An unexpected error occurred. Please try again.'
        showError(message)
        return { handled: true }
    }

    /**
     * Handle success messages
     */
    const handleSuccess = (message: string): void => {
        showSuccess(message)
    }

    /**
     * Handle warning messages
     */
    const handleWarning = (message: string): void => {
        showWarning(message)
    }

    /**
     * Extract field errors from a ValidationError
     */
    const extractFieldErrors = (error: ValidationError): Record<string, string> => {
        const fieldErrors: Record<string, string> = {}
        for (const violation of error.violations) {
            fieldErrors[violation.field] = violation.message
        }
        return fieldErrors
    }

    /**
     * Check if an error is a validation error
     */
    const isValidationError = (error: unknown): error is ValidationError => {
        return error instanceof ValidationError
    }

    /**
     * Handle API errors with translation
     */
    const handleApiError = async (
        error: unknown,
        operation: string,
        context?: string
    ): Promise<void> => {
        const result = handleError(error, t(`errors.${operation}`) || `${operation} failed`)
        if (!result.handled && context) {
            console.error(`${operation} failed:`, { error, context })
        }
    }

    return {
        handleError,
        handleSuccess,
        handleWarning,
        extractFieldErrors,
        isValidationError,
        handleApiError,
    }
}
