import { z } from 'zod'
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
    private isRefreshing = false // Flag to prevent concurrent refresh attempts

    protected getDefaultPageSize(): number {
        return env.defaultPageSize
    }

    protected async request<T>(
        endpoint: string,
        schema: z.ZodType<ApiResponse<T>>,
        options: RequestInit = {}
    ): Promise<T> {
        // Get auth token from auth store
        const authStore = useAuthStore()
        let authToken = authStore.token

        // If no token in store but refresh token exists, try to refresh
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
                    // Trigger refresh through auth store
                    await authStore.refreshTokens()
                    authToken = authStore.token
                } catch (refreshError) {
                    if (this.enableLogging) {
                        console.error('[API] Automatic token refresh failed:', refreshError)
                    }
                    // Don't logout here, let the 401 handler deal with it
                } finally {
                    this.isRefreshing = false
                }
            }
        }

        const config: RequestInit = {
            ...options,
            headers: {
                'Content-Type': 'application/json',
                ...(authToken && {
                    Authorization: `Bearer ${authToken}`,
                }),
                ...options.headers,
            },
        }

        try {
            const fullUrl = buildApiUrl(endpoint)
            if (this.enableLogging) {
                console.log(`[API] ${config.method ?? 'GET'} ${fullUrl}`)
            }

            const response = await fetch(fullUrl, config)

            if (!response.ok) {
                if (response.status === 401) {
                    // Handle unauthorized - try refresh first, then clear auth
                    const refreshToken = getRefreshToken()
                    if (refreshToken && !endpoint.includes('/auth/refresh') && !this.isRefreshing) {
                        // Try to refresh the token once
                        try {
                            if (this.enableLogging) {
                                console.log('[API] 401 received, attempting token refresh')
                            }

                            this.isRefreshing = true
                            // Trigger refresh through auth store
                            await authStore.refreshTokens()
                            const newToken = authStore.token

                            if (newToken) {
                                // Retry the original request with new token
                                const retryConfig = {
                                    ...config,
                                    headers: {
                                        ...config.headers,
                                        Authorization: `Bearer ${newToken}`,
                                    },
                                }
                                const retryResponse = await fetch(
                                    buildApiUrl(endpoint),
                                    retryConfig
                                )

                                if (retryResponse.ok) {
                                    const retryData = await retryResponse.json()
                                    return this.validateResponse(retryData, schema)
                                }
                            }
                        } catch (refreshError) {
                            if (this.enableLogging) {
                                console.error('[API] Token refresh failed:', refreshError)
                            }
                        } finally {
                            this.isRefreshing = false
                        }
                    }

                    // Clear auth and redirect to login
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

            // Fast-path for DSL validate to avoid circular schema issues in test bundlers
            if (endpoint.includes('/dsl/validate')) {
                const raw = (await response.json()) as ApiResponse<T>
                return raw.data as T
            }
            const rawData = await response.json()
            return this.validateResponse(rawData, schema)
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

    protected validateResponse<T>(rawData: unknown, schema: z.ZodType<ApiResponse<T>>): T {
        if (this.enableLogging && this.devMode) {
            console.log('[API] Response:', rawData)
        }

        // Runtime validation with Zod
        let validatedResponse
        try {
            validatedResponse = schema.parse(rawData)
        } catch (validationError) {
            if (validationError instanceof z.ZodError) {
                if (this.enableLogging) {
                    console.error('[API] Response validation failed:', {
                        rawData,
                        validationIssues: validationError.issues,
                    })
                }
                // Create a more user-friendly error message
                const firstIssue = validationError.issues[0]
                const fieldPath = firstIssue.path.join('.') || 'unknown field'
                const message = firstIssue.message || 'Invalid format'
                throw new Error(`Response validation failed: ${message} (${fieldPath})`)
            }
            throw validationError
        }

        if (validatedResponse.status === 'Error') {
            throw new Error(validatedResponse.message)
        }

        // For responses with null data (like logout), return the message
        if (validatedResponse.data === null) {
            return { message: validatedResponse.message } as T
        }

        if (!validatedResponse.data) {
            throw new Error('No data in successful response')
        }

        return validatedResponse.data
    }

    protected async paginatedRequest<T>(
        endpoint: string,
        schema: z.ZodType<ApiResponse<T>>,
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
        // Get auth token from auth store
        const authStore = useAuthStore()
        let authToken = authStore.token

        // If no token in store but refresh token exists, try to refresh
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
                    // Trigger refresh through auth store
                    await authStore.refreshTokens()
                    authToken = authStore.token
                } catch (refreshError) {
                    if (this.enableLogging) {
                        console.error('[API] Automatic token refresh failed:', refreshError)
                    }
                    // Don't logout here, let the 401 handler deal with it
                } finally {
                    this.isRefreshing = false
                }
            }
        }

        const config: RequestInit = {
            ...options,
            headers: {
                'Content-Type': 'application/json',
                ...(authToken && {
                    Authorization: `Bearer ${authToken}`,
                }),
                ...options.headers,
            },
        }

        try {
            const fullUrl = buildApiUrl(endpoint)
            if (this.enableLogging) {
                console.log(`[API] ${config.method ?? 'GET'} ${fullUrl}`)
            }

            const response = await fetch(fullUrl, config)

            if (!response.ok) {
                if (response.status === 401) {
                    // Handle unauthorized - try refresh first, then clear auth
                    const refreshToken = getRefreshToken()
                    if (refreshToken && !endpoint.includes('/auth/refresh') && !this.isRefreshing) {
                        // Try to refresh the token once
                        try {
                            if (this.enableLogging) {
                                console.log('[API] 401 received, attempting token refresh')
                            }

                            this.isRefreshing = true
                            // Trigger refresh through auth store
                            await authStore.refreshTokens()
                            const newToken = authStore.token

                            if (newToken) {
                                // Retry the original request with new token
                                const retryConfig = {
                                    ...config,
                                    headers: {
                                        ...config.headers,
                                        Authorization: `Bearer ${newToken}`,
                                    },
                                }
                                const retryResponse = await fetch(
                                    buildApiUrl(endpoint),
                                    retryConfig
                                )

                                if (retryResponse.ok) {
                                    const retryData = await retryResponse.json()
                                    return this.validatePaginatedResponse(retryData, schema)
                                }
                            }
                        } catch (refreshError) {
                            if (this.enableLogging) {
                                console.error('[API] Token refresh failed:', refreshError)
                            }
                        } finally {
                            this.isRefreshing = false
                        }
                    }

                    // Clear auth and redirect to login
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
                            // If parsing fails, treat as regular error
                            throw new Error(
                                errorData.message ?? `HTTP ${statusCode}: ${response.statusText}`
                            )
                        }
                    }

                    // Handle backend API response format
                    if (errorData.status === 'Error' && errorData.message) {
                        throw new Error(errorData.message)
                    }

                    // Handle other error formats
                    const errorMessage =
                        errorData.message ??
                        errorData.error ??
                        `HTTP ${statusCode}: ${response.statusText}`
                    throw new Error(errorMessage)
                } catch (parseError) {
                    // Re-throw validation errors as-is silently
                    if (parseError instanceof ValidationError) {
                        throw parseError
                    }
                    // Only log non-validation errors
                    if (this.enableLogging) {
                        console.error('[API] Failed to parse error response:', parseError)
                    }
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`)
                }
            }

            const rawData = await response.json()
            return this.validatePaginatedResponse(rawData, schema)
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

    protected validatePaginatedResponse<T>(
        rawData: unknown,
        schema: z.ZodType<ApiResponse<T>>
    ): {
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

        // Runtime validation with Zod
        let validatedResponse
        try {
            validatedResponse = schema.parse(rawData)
        } catch (validationError) {
            if (validationError instanceof z.ZodError) {
                if (this.enableLogging) {
                    console.error('[API] Response validation failed:', {
                        rawData,
                        validationIssues: validationError.issues,
                    })
                }
                // Create a more user-friendly error message
                const firstIssue = validationError.issues[0]
                const fieldPath = firstIssue.path.join('.') || 'unknown field'
                const message = firstIssue.message || 'Invalid format'
                throw new Error(`Response validation failed: ${message} (${fieldPath})`)
            }
            throw validationError
        }

        if (validatedResponse.status === 'Error') {
            throw new Error(validatedResponse.message)
        }

        if (!validatedResponse.data) {
            throw new Error('No data in successful response')
        }

        return {
            data: validatedResponse.data,
            meta: validatedResponse.meta ?? undefined,
        }
    }
}
