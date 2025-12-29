import { useSnackbar } from './useSnackbar'
import { useTranslations } from './useTranslations'
import { ValidationError } from '@/api/typed-client'
import { HttpError } from '@/api/errors'

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
    const handleError = (error: unknown, context?: string): ErrorHandlerResult => {
        // Handle ValidationError specifically
        if (error instanceof ValidationError) {
            // Convert violations to a field error map
            const fieldErrors: Record<string, string> = {}
            for (const violation of error.violations) {
                fieldErrors[violation.field] = violation.message
            }

            // Show general error message
            // Try to get namespace from context if available
            const namespace = context ? context.split('.')[0] : 'unknown'
            const messageKey = `error.${namespace}.422`
            let message = t(messageKey)
            if (!message || message === messageKey) {
                message = t('error.422') || 'Validation failed'
            }
            if (!message || message === 'error.422') {
                message = error.message || 'Validation failed'
            }

            showError(message)

            return {
                handled: true,
                fieldErrors,
            }
        }

        // Handle HttpError with namespace + action + status code
        if (error instanceof HttpError) {
            const { statusCode, namespace, action, originalMessage } = error
            const finalNamespace = namespace ?? context ?? 'unknown' // Use provided context if HttpError doesn't have one

            // For 422 errors, try to translate the backend message
            if (statusCode === 422 && originalMessage) {
                // Try to find translation for the specific error message
                // Format: error.{namespace}.422.{sanitized_message}
                // Example: "Username is required" → error.user.422.username_is_required
                const sanitizedMessage = originalMessage
                    .toLowerCase()
                    .replace(/[^a-z0-9\s]/g, '')
                    .replace(/\s+/g, '_')
                const messageKey = `error.${finalNamespace}.422.${sanitizedMessage}`
                const message = t(messageKey)
                if (message && message !== messageKey) {
                    showError(message)
                    return { handled: true }
                }
                // Fallback to namespace-specific 422 message, then generic
            }

            // Try most specific key first: error.{namespace}.{action}.{statusCode}
            let translationKey = `error.${finalNamespace}.${action}.${statusCode}`
            let message = t(translationKey)

            // Fallback to namespace-specific key: error.{namespace}.{statusCode}
            if (!message || message === translationKey) {
                translationKey = `error.${finalNamespace}.${statusCode}`
                message = t(translationKey)
            }

            // Fallback to generic status code key: error.{statusCode}
            if (!message || message === translationKey) {
                translationKey = `error.${statusCode}`
                message = t(translationKey)
            }

            // Final fallback
            if (!message || message === translationKey) {
                message = t('error.generic') || 'An error occurred'
            }

            showError(message) // Never show HTTP codes
            return { handled: true }
        }

        // Handle regular Error instances (fallback for non-HTTP errors)
        if (error instanceof Error) {
            const errorMessage = error.message

            // Check for network errors
            if (errorMessage.includes('network') || errorMessage.includes('connection')) {
                showError(
                    t('error.network') || t('error.generic') || 'Network error. Please try again.'
                )
                return { handled: true }
            }

            // Generic error
            showError(errorMessage || t('error.generic') || 'An error occurred')
            return { handled: true }
        }

        // Handle unknown error types
        const message = t('error.generic') || 'An unexpected error occurred. Please try again.'
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
