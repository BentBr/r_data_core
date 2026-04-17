import { env, buildApiUrl } from '@/env-check'
import { useAuthStore } from '@/stores/auth'
import { getRefreshToken } from '@/utils/cookies'
import { ValidationErrorResponseSchema } from '@/types/schemas'
import type { Meta } from '@/types/schemas'
import { ValidationError } from '../http-client'
import { HttpError, extractNamespaceFromEndpoint, extractActionFromMethod } from '../errors'

// Define ApiResponse type for backward compatibility
export type ApiResponse<T> = {
    status: 'Success' | 'Error'
    message: string
    data?: T | null
    meta?: Meta | null // Backend may return null
}

/**
 * Base class for typed HTTP clients
 * Provides core request handling, authentication, and validation
 */
export class BaseTypedHttpClient {
    protected enableLogging = env.enableApiLogging
    protected devMode = env.devMode
    private isRefreshing = false // Re-entrancy guard for token refresh

    protected getDefaultPageSize(): number {
        return env.defaultPageSize
    }

    /**
     * Build request config with auth header
     */
    private buildConfig(authToken: string | null, options: RequestInit): RequestInit {
        return {
            ...options,
            headers: {
                'Content-Type': 'application/json',
                ...(authToken && {
                    Authorization: `Bearer ${authToken}`,
                }),
                ...options.headers,
            },
        }
    }

    /**
     * Attempt pre-request token refresh when no access token exists but a refresh token is available
     */
    private async ensureToken(endpoint: string): Promise<string | null> {
        const authStore = useAuthStore()
        let authToken = authStore.token

        if (!authToken) {
            const refreshToken = getRefreshToken()
            if (refreshToken && !endpoint.includes('/auth/refresh') && !this.isRefreshing) {
                if (this.enableLogging) {
                    console.log(
                        '[API] No access token but refresh token exists, attempting refresh'
                    )
                }
                try {
                    this.isRefreshing = true
                    await authStore.refreshTokens()
                    authToken = authStore.token
                } catch (refreshError) {
                    if (this.enableLogging) {
                        console.error('[API] Automatic token refresh failed:', refreshError)
                    }
                } finally {
                    this.isRefreshing = false
                }
            }
        }

        return authToken
    }

    /**
     * Handle 401 response by refreshing the token and retrying the request.
     * Returns the successful retry Response, or null if retry failed.
     *
     * When isRefreshing is true (another request owns the refresh), we wait for
     * the auth store's shared refresh promise instead of skipping to logout.
     */
    private async handleUnauthorized(
        config: RequestInit,
        endpoint: string
    ): Promise<Response | null> {
        const refreshToken = getRefreshToken()
        if (!refreshToken || endpoint.includes('/auth/refresh')) {
            return null
        }

        const authStore = useAuthStore()

        if (this.isRefreshing) {
            // Another request is already refreshing — wait via the auth store's shared promise
            try {
                await authStore.refreshTokens()
            } catch (refreshError) {
                if (this.enableLogging) {
                    console.error('[API] Token refresh failed (concurrent):', refreshError)
                }
                return null
            }
        } else {
            // Primary refresh path — we own the refresh
            try {
                if (this.enableLogging) {
                    console.log('[API] 401 received, attempting token refresh')
                }
                this.isRefreshing = true
                await authStore.refreshTokens()
            } catch (refreshError) {
                if (this.enableLogging) {
                    console.error('[API] Token refresh failed:', refreshError)
                }
                return null
            } finally {
                this.isRefreshing = false
            }
        }

        const newToken = authStore.token
        if (!newToken) {
            return null
        }

        // Retry the original request with the new token
        const retryConfig = {
            ...config,
            headers: {
                ...config.headers,
                Authorization: `Bearer ${newToken}`,
            },
        }
        const retryResponse = await fetch(buildApiUrl(endpoint), retryConfig)
        return retryResponse.ok ? retryResponse : null
    }

    protected async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
        const authToken = await this.ensureToken(endpoint)
        const config = this.buildConfig(authToken, options)

        try {
            const fullUrl = buildApiUrl(endpoint)
            if (this.enableLogging) {
                console.log(`[API] ${config.method ?? 'GET'} ${fullUrl}`)
            }

            const response = await fetch(fullUrl, config)

            if (!response.ok) {
                if (response.status === 401) {
                    const retryResponse = await this.handleUnauthorized(config, endpoint)
                    if (retryResponse) {
                        const retryData = await retryResponse.json()
                        return this.validateResponse(retryData)
                    }

                    // All refresh attempts failed — clear auth
                    const authStore = useAuthStore()
                    await authStore.logout()
                    const namespace = extractNamespaceFromEndpoint(endpoint)
                    const action = extractActionFromMethod(options.method)
                    throw new HttpError(
                        401,
                        namespace,
                        action,
                        'Authentication required',
                        'Authentication required'
                    )
                }

                // Try to extract error message from response
                try {
                    const errorData = await response.json()
                    const statusCode = response.status

                    // Determine if this is an expected/handled error (not a true error)
                    const isExpectedError = [400, 409, 422].includes(statusCode)

                    // Only log as error for unexpected status codes; expected ones are handled gracefully
                    if (this.enableLogging && !isExpectedError) {
                        console.error('[API] HTTP Error Response:', {
                            status: statusCode,
                            statusText: response.statusText,
                            errorData,
                            endpoint,
                        })
                    } else if (this.enableLogging && this.devMode) {
                        // In dev mode, log expected errors as info for debugging
                        console.log('[API] Handled HTTP Response:', {
                            status: statusCode,
                            statusText: response.statusText,
                            errorData,
                            endpoint,
                        })
                    }

                    const namespace = extractNamespaceFromEndpoint(endpoint)
                    const action = extractActionFromMethod(options.method)

                    // Handle validation errors (422) and bad request errors (400) with structured violations
                    if ((statusCode === 422 || statusCode === 400) && errorData.violations) {
                        try {
                            const validationError = ValidationErrorResponseSchema.parse(errorData)
                            throw new ValidationError(
                                validationError.message,
                                validationError.violations
                            )
                        } catch (parseError) {
                            // Re-throw ValidationError as-is
                            if (parseError instanceof ValidationError) {
                                throw parseError
                            }
                            // If parsing fails, log and treat as regular error
                            if (this.enableLogging) {
                                console.error('[API] Failed to parse validation error:', parseError)
                            }
                            const message = errorData.message ?? response.statusText
                            throw new HttpError(statusCode, namespace, action, message, message)
                        }
                    }

                    // Handle 400 errors that might have validation-like structure
                    if (statusCode === 400 && errorData.message) {
                        // Try to extract field errors from the message (Symfony style)
                        const message = errorData.message
                        // Check if it's a deserialization error that can be converted to validation error
                        if (
                            message.includes('unknown variant') ||
                            message.includes('Json deserialize error')
                        ) {
                            // Extract field name from error message if possible
                            const fieldMatch = message.match(/unknown variant `(\w+)`/i)
                            const field = fieldMatch ? fieldMatch[1] : 'dsl'
                            throw new ValidationError('Validation failed', [
                                {
                                    field,
                                    message: message.replace(/Json deserialize error: /, ''),
                                    code: 'INVALID_VALUE',
                                },
                            ])
                        }
                    }

                    // Handle backend API response format
                    if (errorData.status === 'Error' && errorData.message) {
                        const message = errorData.message
                        throw new HttpError(statusCode, namespace, action, message, message)
                    }

                    // Handle other error formats
                    const message = errorData.message ?? errorData.error ?? response.statusText
                    throw new HttpError(statusCode, namespace, action, message, message)
                } catch (parseError) {
                    // Re-throw validation errors as-is silently
                    if (parseError instanceof ValidationError) {
                        throw parseError
                    }
                    // Re-throw HttpError as-is
                    if (parseError instanceof HttpError) {
                        throw parseError
                    }
                    // Only log non-validation errors
                    if (this.enableLogging) {
                        console.error('[API] Failed to parse error response:', parseError)
                    }
                    const namespace = extractNamespaceFromEndpoint(endpoint)
                    const action = extractActionFromMethod(options.method)
                    throw new HttpError(
                        response.status,
                        namespace,
                        action,
                        response.statusText,
                        response.statusText
                    )
                }
            }

            const rawData = await response.json()
            return this.validateResponse(rawData)
        } catch (error) {
            // Don't log expected errors (ValidationError, HttpError) to console as they're handled behavior
            if (!(error instanceof ValidationError) && !(error instanceof HttpError)) {
                if (this.enableLogging) {
                    console.error('[API] Error:', {
                        error: error instanceof Error ? error.message : error,
                        endpoint,
                        stack: error instanceof Error ? error.stack : undefined,
                    })
                }
            }
            throw error
        }
    }

    protected validateResponse<T>(rawData: unknown): T {
        if (this.enableLogging && this.devMode) {
            console.log('[API] Response:', rawData)
        }

        const response = rawData as ApiResponse<T>

        if (response.status === 'Error') {
            throw new Error(response.message)
        }

        // For responses with null data (like logout), return the message
        if (response.data === null) {
            return { message: response.message } as T
        }

        if (!response.data) {
            throw new Error('No data in successful response')
        }

        return response.data
    }

    protected async paginatedRequest<T>(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<{
        data: T
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
            request_id?: string
            timestamp?: string
            custom?: unknown
        }
    }> {
        const authToken = await this.ensureToken(endpoint)
        const config = this.buildConfig(authToken, options)

        try {
            const fullUrl = buildApiUrl(endpoint)
            if (this.enableLogging) {
                console.log(`[API] ${config.method ?? 'GET'} ${fullUrl}`)
            }

            const response = await fetch(fullUrl, config)

            if (!response.ok) {
                if (response.status === 401) {
                    const retryResponse = await this.handleUnauthorized(config, endpoint)
                    if (retryResponse) {
                        const retryData = await retryResponse.json()
                        return this.validatePaginatedResponse(retryData)
                    }

                    // All refresh attempts failed — clear auth
                    const authStore = useAuthStore()
                    await authStore.logout()
                    throw new Error('Authentication required')
                }

                // Try to extract error message from response
                try {
                    const errorData = await response.json()
                    const statusCode = response.status

                    // Determine if this is an expected/handled error (not a true error)
                    const isExpectedError = [400, 409, 422].includes(statusCode)

                    // Only log as error for unexpected status codes; expected ones are handled gracefully
                    if (this.enableLogging && !isExpectedError) {
                        console.error('[API] HTTP Error Response:', {
                            status: statusCode,
                            statusText: response.statusText,
                            errorData,
                            endpoint,
                        })
                    } else if (this.enableLogging && this.devMode) {
                        // In dev mode, log expected errors as info for debugging
                        console.log('[API] Handled HTTP Response:', {
                            status: statusCode,
                            statusText: response.statusText,
                            errorData,
                            endpoint,
                        })
                    }

                    const namespace = extractNamespaceFromEndpoint(endpoint)
                    const action = extractActionFromMethod(options.method)

                    // Handle validation errors (422) with structured violations
                    if (statusCode === 422 && errorData.violations) {
                        try {
                            const validationError = ValidationErrorResponseSchema.parse(errorData)
                            throw new ValidationError(
                                validationError.message,
                                validationError.violations
                            )
                        } catch (parseError) {
                            // Re-throw ValidationError as-is
                            if (parseError instanceof ValidationError) {
                                throw parseError
                            }
                            // If parsing fails, log and treat as regular error
                            if (this.enableLogging) {
                                console.error('[API] Failed to parse validation error:', parseError)
                            }
                            const message = errorData.message ?? response.statusText
                            throw new HttpError(statusCode, namespace, action, message, message)
                        }
                    }

                    // Handle backend API response format
                    if (errorData.status === 'Error' && errorData.message) {
                        const message = errorData.message
                        throw new HttpError(statusCode, namespace, action, message, message)
                    }

                    // Handle other error formats
                    const message = errorData.message ?? errorData.error ?? response.statusText
                    throw new HttpError(statusCode, namespace, action, message, message)
                } catch (parseError) {
                    // Re-throw validation errors as-is silently
                    if (parseError instanceof ValidationError) {
                        throw parseError
                    }
                    // Re-throw HttpError as-is
                    if (parseError instanceof HttpError) {
                        throw parseError
                    }
                    // Only log non-validation errors
                    if (this.enableLogging) {
                        console.error('[API] Failed to parse error response:', parseError)
                    }
                    const namespace = extractNamespaceFromEndpoint(endpoint)
                    const action = extractActionFromMethod(options.method)
                    throw new HttpError(
                        response.status,
                        namespace,
                        action,
                        response.statusText,
                        response.statusText
                    )
                }
            }

            const rawData = await response.json()
            return this.validatePaginatedResponse(rawData)
        } catch (error) {
            // Don't log expected errors (ValidationError, HttpError) to console as they're handled behavior
            if (!(error instanceof ValidationError) && !(error instanceof HttpError)) {
                if (this.enableLogging) {
                    console.error('[API] Error:', {
                        error: error instanceof Error ? error.message : error,
                        endpoint,
                        stack: error instanceof Error ? error.stack : undefined,
                    })
                }
            }
            throw error
        }
    }

    protected validatePaginatedResponse<T>(rawData: unknown): {
        data: T
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
            request_id?: string
            timestamp?: string
            custom?: unknown
        }
    } {
        if (this.enableLogging && this.devMode) {
            console.log('[API] Response:', rawData)
        }

        const response = rawData as ApiResponse<T>

        if (response.status === 'Error') {
            throw new Error(response.message)
        }

        if (!response.data) {
            throw new Error('No data in successful response')
        }

        return {
            data: response.data,
            meta: response.meta ?? undefined,
        }
    }
}
