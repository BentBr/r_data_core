import { z } from 'zod'
import { env } from '@/env-check'
import {
    ApiResponseSchema,
    ClassDefinitionSchema,
    ApiKeySchema,
    UserSchema,
    LoginResponseSchema,
    RefreshTokenResponseSchema,
    type ApiResponse,
    type ClassDefinition,
    type ApiKey,
    type User,
    type LoginResponse,
    type LoginRequest,
    type RefreshTokenRequest,
    type RefreshTokenResponse,
} from '@/types/schemas'

class TypedHttpClient {
    private baseURL = env.apiBaseUrl
    private enableLogging = env.enableApiLogging
    private devMode = env.devMode

    async request<T>(
        endpoint: string,
        schema: z.ZodType<ApiResponse<T>>,
        options: RequestInit = {}
    ): Promise<T> {
        // Get auth token from localStorage (auth store will handle this)
        const authToken = localStorage.getItem('auth_token')

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
                console.log(`[API] ${config.method || 'GET'} ${this.baseURL}${endpoint}`)
            }

            const response = await fetch(`${this.baseURL}${endpoint}`, config)

            if (!response.ok) {
                if (response.status === 401) {
                    // Handle unauthorized - try refresh first, then clear auth
                    const refreshToken = localStorage.getItem('refresh_token')
                    if (refreshToken && !endpoint.includes('/auth/refresh')) {
                        // Try to refresh the token once
                        try {
                            const refreshResponse = await this.refreshToken({
                                refresh_token: refreshToken,
                            })

                            // Update stored tokens
                            localStorage.setItem('auth_token', refreshResponse.access_token)
                            localStorage.setItem('refresh_token', refreshResponse.refresh_token)

                            // Retry the original request with new token
                            const retryConfig = {
                                ...config,
                                headers: {
                                    ...config.headers,
                                    Authorization: `Bearer ${refreshResponse.access_token}`,
                                },
                            }
                            const retryResponse = await fetch(
                                `${this.baseURL}${endpoint}`,
                                retryConfig
                            )

                            if (retryResponse.ok) {
                                return await retryResponse.json()
                            }
                        } catch (refreshError) {
                            if (this.enableLogging) {
                                console.error('[API] Token refresh failed:', refreshError)
                            }
                        }
                    }

                    // Clear auth and redirect to login
                    localStorage.removeItem('auth_token')
                    localStorage.removeItem('refresh_token')
                    if (window.location.pathname !== '/login') {
                        window.location.href = '/login'
                    }
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

                    // Handle backend API response format
                    if (errorData.status === 'Error' && errorData.message) {
                        throw new Error(errorData.message)
                    }

                    // Handle other error formats
                    const errorMessage =
                        errorData.message ||
                        errorData.error ||
                        `HTTP ${response.status}: ${response.statusText}`
                    throw new Error(errorMessage)
                } catch (parseError) {
                    if (this.enableLogging) {
                        console.error('[API] Failed to parse error response:', parseError)
                    }
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`)
                }
            }

            const rawData = await response.json()

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
                            endpoint,
                        })
                    }
                    // Create a more user-friendly error message
                    const firstIssue = validationError.issues[0]
                    const fieldPath = firstIssue?.path?.join('.') || 'unknown field'
                    const message = firstIssue?.message || 'Invalid format'
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

            return validatedResponse.data
        } catch (error) {
            if (this.enableLogging) {
                console.error('[API] Error:', {
                    error: error instanceof Error ? error.message : error,
                    endpoint,
                    stack: error instanceof Error ? error.stack : undefined,
                })
            }
            throw error
        }
    }

    // Type-safe API methods with runtime validation
    private getDefaultPageSize(): number {
        return env.defaultPageSize
    }

    async getClassDefinitions(limit?: number, offset = 0): Promise<ClassDefinition[]> {
        const pageSize = limit ?? this.getDefaultPageSize()
        return this.request(
            `/admin/api/v1/class-definitions?limit=${pageSize}&offset=${offset}`,
            ApiResponseSchema(z.array(ClassDefinitionSchema))
        )
    }

    async getClassDefinition(uuid: string): Promise<ClassDefinition> {
        return this.request(
            `/admin/api/v1/class-definitions/${uuid}`,
            ApiResponseSchema(ClassDefinitionSchema)
        )
    }

    async createClassDefinition(data: Partial<ClassDefinition>): Promise<{ uuid: string }> {
        return this.request(
            '/admin/api/v1/class-definitions',
            ApiResponseSchema(z.object({ uuid: z.string().uuid() })),
            {
                method: 'POST',
                body: JSON.stringify(data),
            }
        )
    }

    async updateClassDefinition(
        uuid: string,
        data: Partial<ClassDefinition>
    ): Promise<{ uuid: string }> {
        return this.request(
            `/admin/api/v1/class-definitions/${uuid}`,
            ApiResponseSchema(z.object({ uuid: z.string().uuid() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }

    async deleteClassDefinition(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/class-definitions/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async getApiKeys(limit?: number, offset = 0): Promise<ApiKey[]> {
        const pageSize = limit ?? this.getDefaultPageSize()
        return this.request(
            `/admin/api/v1/api-keys?limit=${pageSize}&offset=${offset}`,
            ApiResponseSchema(z.array(ApiKeySchema))
        )
    }

    async login(credentials: LoginRequest): Promise<LoginResponse> {
        return this.request('/admin/api/v1/auth/login', ApiResponseSchema(LoginResponseSchema), {
            method: 'POST',
            body: JSON.stringify(credentials),
        })
    }

    async getUsers(limit?: number, offset = 0): Promise<User[]> {
        const pageSize = limit ?? this.getDefaultPageSize()
        return this.request(
            `/admin/api/v1/users?limit=${pageSize}&offset=${offset}`,
            ApiResponseSchema(z.array(UserSchema))
        )
    }

    async refreshToken(refreshTokenRequest: RefreshTokenRequest): Promise<RefreshTokenResponse> {
        return this.request(
            '/admin/api/v1/auth/refresh',
            ApiResponseSchema(RefreshTokenResponseSchema),
            {
                method: 'POST',
                body: JSON.stringify(refreshTokenRequest),
            }
        )
    }

    async revokeToken(revokeTokenRequest: { refresh_token: string }): Promise<{ message: string }> {
        return this.request(
            '/admin/api/v1/auth/revoke',
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'POST',
                body: JSON.stringify(revokeTokenRequest),
            }
        )
    }

    async revokeAllTokens(): Promise<{ message: string }> {
        return this.request(
            '/admin/api/v1/auth/revoke-all',
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'POST',
                body: JSON.stringify({}),
            }
        )
    }
}

export const typedHttpClient = new TypedHttpClient()
export type { TypedHttpClient }
