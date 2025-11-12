import { z } from 'zod'
import { env } from '@/env-check'
import { useAuthStore } from '@/stores/auth'
import { getRefreshToken } from '@/utils/cookies'
import { ValidationErrorResponseSchema } from '@/types/schemas'
import type { Meta } from '@/types/schemas'
import { ValidationError } from '../http-client'

// Define ApiResponse type for backward compatibility
export type ApiResponse<T> = {
    status: 'Success' | 'Error'
    message: string
    data?: T
    meta?: Meta
}

/**
 * Base class for typed HTTP clients
 * Provides core request handling, authentication, and validation
 */
export class BaseTypedHttpClient {
    protected baseURL = env.apiBaseUrl
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
            if (this.enableLogging) {
                console.log(`[API] ${config.method ?? 'GET'} ${this.baseURL}${endpoint}`)
            }

            const response = await fetch(`${this.baseURL}${endpoint}`, config)

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
                                    `${this.baseURL}${endpoint}`,
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
                    throw new Error('Authentication required')
                }

                // Try to extract error message from response
                try {
                    const errorData = await response.json()
                    if (this.enableLogging) {
                        console.error('[API] HTTP Error Response:', {
                            status: response.status,
                            statusText: response.statusText,
                            errorData,
                            endpoint,
                        })
                    }

                    // Handle validation errors (422) with structured violations
                    if (response.status === 422 && errorData.violations) {
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
                            throw new Error(
                                errorData.message ??
                                    `HTTP ${response.status}: ${response.statusText}`
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
                        `HTTP ${response.status}: ${response.statusText}`
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

            // Fast-path for DSL validate to avoid circular schema issues in test bundlers
            if (endpoint.includes('/dsl/validate')) {
                const raw = await response.json()
                const data = (raw as any)?.data
                return data as T
            }
            const rawData = await response.json()
            return this.validateResponse(rawData, schema)
        } catch (error) {
            // Don't log validation errors to console as they're expected behavior
            if (!(error instanceof ValidationError)) {
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
                const fieldPath = firstIssue?.path?.join('.') || 'unknown field'
                const message = firstIssue?.message ?? 'Invalid format'
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
            if (this.enableLogging) {
                console.log(`[API] ${config.method ?? 'GET'} ${this.baseURL}${endpoint}`)
            }

            const response = await fetch(`${this.baseURL}${endpoint}`, config)

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
                                    `${this.baseURL}${endpoint}`,
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
                    if (this.enableLogging) {
                        console.error('[API] HTTP Error Response:', {
                            status: response.status,
                            statusText: response.statusText,
                            errorData,
                            endpoint,
                        })
                    }

                    // Handle validation errors (422) with structured violations
                    if (response.status === 422 && errorData.violations) {
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
                                errorData.message ??
                                    `HTTP ${response.status}: ${response.statusText}`
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
                        `HTTP ${response.status}: ${response.statusText}`
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
            // Don't log validation errors to console as they're expected behavior
            if (!(error instanceof ValidationError)) {
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
                const fieldPath = firstIssue?.path?.join('.') || 'unknown field'
                const message = firstIssue?.message ?? 'Invalid format'
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
