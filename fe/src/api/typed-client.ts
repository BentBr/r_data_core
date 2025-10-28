import { z } from 'zod'
import { env } from '@/env-check'
import { useAuthStore } from '@/stores/auth'
import { getRefreshToken } from '@/utils/cookies'
import {
    ApiResponseSchema,
    PaginatedApiResponseSchema,
    LoginResponseSchema,
    RefreshTokenResponseSchema,
    UserSchema,
    ApiKeySchema,
    ApiKeyCreatedResponseSchema,
    EntityDefinitionSchema,
    DynamicEntitySchema,
    // CreateEntityRequestSchema,
    // UpdateEntityRequestSchema,
    EntityResponseSchema,
    ValidationErrorResponseSchema,
} from '@/types/schemas'

// Import ValidationError from http-client.ts
// Eventually this file should be refactored to use the new HttpClient class
import { ValidationError } from './http-client'
export { ValidationError }

// Import types from schemas
import type {
    LoginRequest,
    LoginResponse,
    RefreshTokenRequest,
    RefreshTokenResponse,
    LogoutRequest,
    User,
    ApiKey,
    CreateApiKeyRequest,
    ApiKeyCreatedResponse,
    ReassignApiKeyRequest,
    EntityDefinition,
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
    DynamicEntity,
    CreateEntityRequest,
    UpdateEntityRequest,
    EntityResponse,
    Meta,
} from '@/types/schemas'

// Re-export HttpClient and types for future use
export { HttpClient as TypedHttpClientBase } from './http-client'
export type { ApiResponse } from './http-client'

// Define ApiResponse type for backward compatibility
type ApiResponse<T> = {
    status: 'Success' | 'Error'
    message: string
    data?: T
    meta?: Meta
}

// TODO: Refactor this class to extend HttpClient
class TypedHttpClient {
    private baseURL = env.apiBaseUrl
    private enableLogging = env.enableApiLogging
    private devMode = env.devMode
    private isRefreshing = false // Flag to prevent concurrent refresh attempts

    async request<T>(
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

    private validateResponse<T>(rawData: unknown, schema: z.ZodType<ApiResponse<T>>): T {
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
        if (validatedResponse.data === null ?? validatedResponse.data === undefined) {
            return { message: validatedResponse.message } as T
        }

        if (!validatedResponse.data) {
            throw new Error('No data in successful response')
        }

        return validatedResponse.data
    }

    private async paginatedRequest<T>(
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

    private validatePaginatedResponse<T>(
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

    // Type-safe API methods with runtime validation
    private getDefaultPageSize(): number {
        return env.defaultPageSize
    }

    async getEntityDefinitions(
        limit?: number,
        offset = 0
    ): Promise<{
        data: EntityDefinition[]
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
        const pageSize = limit ?? this.getDefaultPageSize()
        const response = await this.paginatedRequest(
            `/admin/api/v1/entity-definitions?limit=${pageSize}&offset=${offset}`,
            PaginatedApiResponseSchema(z.array(EntityDefinitionSchema))
        )
        return response
    }

    async getEntityDefinition(uuid: string): Promise<EntityDefinition> {
        return this.request(
            `/admin/api/v1/entity-definitions/${uuid}`,
            ApiResponseSchema(EntityDefinitionSchema)
        )
    }

    async createEntityDefinition(data: CreateEntityDefinitionRequest): Promise<{ uuid: string }> {
        return this.request(
            '/admin/api/v1/entity-definitions',
            ApiResponseSchema(z.object({ uuid: z.string().uuid() })),
            {
                method: 'POST',
                body: JSON.stringify(data),
            }
        )
    }

    async updateEntityDefinition(
        uuid: string,
        data: UpdateEntityDefinitionRequest
    ): Promise<{ uuid: string }> {
        return this.request(
            `/admin/api/v1/entity-definitions/${uuid}`,
            ApiResponseSchema(z.object({ uuid: z.string().uuid() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }

    async deleteEntityDefinition(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/entity-definitions/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async applyEntityDefinitionSchema(uuid?: string): Promise<{ message: string }> {
        const endpoint = uuid
            ? '/admin/api/v1/entity-definitions/apply-schema'
            : '/admin/api/v1/entity-definitions/apply-schema'

        return this.request(endpoint, ApiResponseSchema(z.object({ message: z.string() })), {
            method: 'POST',
            body: JSON.stringify({ uuid }),
        })
    }

    async getApiKeys(
        page = 1,
        itemsPerPage = 10
    ): Promise<{
        data: ApiKey[]
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
        const response = await this.paginatedRequest(
            `/admin/api/v1/api-keys?page=${page}&per_page=${itemsPerPage}`,
            PaginatedApiResponseSchema(z.array(ApiKeySchema))
        )
        return response
    }

    async createApiKey(data: CreateApiKeyRequest): Promise<ApiKeyCreatedResponse> {
        return this.request(
            '/admin/api/v1/api-keys',
            ApiResponseSchema(ApiKeyCreatedResponseSchema),
            {
                method: 'POST',
                body: JSON.stringify(data),
            }
        )
    }

    async revokeApiKey(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/api-keys/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async reassignApiKey(uuid: string, data: ReassignApiKeyRequest): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/api-keys/${uuid}/reassign`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
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

    async logout(logoutRequest: LogoutRequest): Promise<{ message: string }> {
        const result = await this.request(
            '/admin/api/v1/auth/logout',
            ApiResponseSchema(z.null()),
            {
                method: 'POST',
                body: JSON.stringify(logoutRequest),
            }
        )
        return result as unknown as { message: string }
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

    // Entity methods
    async getEntities(
        entityType: string,
        page = 1,
        itemsPerPage = 10,
        include?: string
    ): Promise<{
        data: DynamicEntity[]
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
        const includeParam = include ? `&include=${include}` : ''
        const response = await this.paginatedRequest(
            `/api/v1/${entityType}?page=${page}&per_page=${itemsPerPage}${includeParam}`,
            PaginatedApiResponseSchema(z.array(DynamicEntitySchema))
        )
        return response
    }

    // Browse by virtual path (folders first, then files)
    async browseByPath(
        path: string,
        limit = this.getDefaultPageSize(),
        offset = 0
    ): Promise<{
        data: Array<{
            kind: 'folder' | 'file'
            name: string
            path: string
            entity_uuid?: string
            entity_type?: string
            has_children?: boolean
        }>
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
        const encoded = encodeURIComponent(path)
        const response = await this.paginatedRequest(
            `/api/v1/entities/by-path?path=${encoded}&limit=${limit}&offset=${offset}`,
            PaginatedApiResponseSchema(
                z.array(
                    z.object({
                        kind: z.enum(['folder', 'file']),
                        name: z.string(),
                        path: z.string(),
                        entity_uuid: z.string().uuid().nullable().optional(),
                        entity_type: z.string().nullable().optional(),
                        has_children: z.boolean().nullable().optional(),
                    })
                )
            )
        )
        return response
    }

    // Query entities by parent or path
    async queryEntities(
        entityType: string,
        options: {
            parentUuid?: string
            path?: string
            limit?: number
            offset?: number
        }
    ): Promise<DynamicEntity[]> {
        const { parentUuid, path, limit = 20, offset = 0 } = options
        return this.request(
            '/api/v1/entities/query',
            ApiResponseSchema(z.array(DynamicEntitySchema)),
            {
                method: 'POST',
                body: JSON.stringify({
                    entity_type: entityType,
                    parent_uuid: parentUuid,
                    path: path,
                    limit: limit,
                    offset: offset,
                }),
            }
        )
    }

    async getEntity(entityType: string, uuid: string): Promise<DynamicEntity> {
        return this.request(`/api/v1/${entityType}/${uuid}`, ApiResponseSchema(DynamicEntitySchema))
    }

    async createEntity(
        entityType: string,
        data: CreateEntityRequest
    ): Promise<EntityResponse | null> {
        const body: Record<string, unknown> = { ...data.data }
        if (data.parent_uuid !== undefined && data.parent_uuid !== null) {
            body.parent_uuid = data.parent_uuid
        }
        return this.request(
            `/api/v1/${entityType}`,
            ApiResponseSchema(EntityResponseSchema) as z.ZodType<
                ApiResponse<EntityResponse | null>
            >,
            {
                method: 'POST',
                body: JSON.stringify(body),
            }
        )
    }

    async updateEntity(
        entityType: string,
        uuid: string,
        data: UpdateEntityRequest
    ): Promise<EntityResponse | null> {
        const body: Record<string, unknown> = { ...data.data }
        if (data.parent_uuid !== undefined) {
            body.parent_uuid = data.parent_uuid
        }
        return this.request(
            `/api/v1/${entityType}/${uuid}`,
            ApiResponseSchema(EntityResponseSchema) as z.ZodType<
                ApiResponse<EntityResponse | null>
            >,
            {
                method: 'PUT',
                body: JSON.stringify(body),
            }
        )
    }

    async deleteEntity(entityType: string, uuid: string): Promise<{ message: string }> {
        return this.request(
            `/api/v1/${entityType}/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }
}

export const typedHttpClient = new TypedHttpClient()
export type { TypedHttpClient }
