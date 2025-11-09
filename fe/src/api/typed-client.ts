import { z } from 'zod'
import { env } from '@/env-check'
import { useAuthStore } from '@/stores/auth'
import { getRefreshToken } from '@/utils/cookies'
import { ApiResponseSchema, PaginatedApiResponseSchema, LoginResponseSchema, RefreshTokenResponseSchema, UserSchema, ApiKeySchema, ApiKeyCreatedResponseSchema, EntityDefinitionSchema, DynamicEntitySchema, EntityResponseSchema, ValidationErrorResponseSchema, UuidSchema } from '@/types/schemas'

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
        if (validatedResponse.data === null) {
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
            // todo add validationError and catch it (don't log it)
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
            ApiResponseSchema(z.object({ uuid: UuidSchema })),
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
            ApiResponseSchema(z.object({ uuid: UuidSchema })),
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

    // Workflows (Admin)
    async listWorkflows(): Promise<
        Array<{
            uuid: string
            name: string
            kind: string
            enabled: boolean
            schedule_cron?: string | null
        }>
    > {
        // Use loose schema to avoid strict typing issues in legacy client
        const data = await this.request('/admin/api/v1/workflows', ApiResponseSchema(z.any()))
        return data as Array<{
            uuid: string
            name: string
            kind: string
            enabled: boolean
            schedule_cron?: string | null
        }>
    }

    async runWorkflow(uuid: string): Promise<{ message: string }> {
        const data = await this.request(
            `/admin/api/v1/workflows/${uuid}/run`,
            ApiResponseSchema(z.any()),
            { method: 'POST' }
        )
        return data as { message: string }
    }

    async createWorkflow(data: {
        name: string
        description?: string | null
        kind: 'consumer' | 'provider'
        enabled: boolean
        schedule_cron?: string | null
        config: unknown
    }): Promise<{ uuid: string }> {
        const Schema = z.object({ uuid: UuidSchema })
        return this.request('/admin/api/v1/workflows', ApiResponseSchema(Schema), {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async updateWorkflow(
        uuid: string,
        data: {
            name: string
            description?: string | null
            kind: 'consumer' | 'provider'
            enabled: boolean
            schedule_cron?: string | null
            config: unknown
        }
    ): Promise<{ message: string }> {
        const Schema = z.object({ message: z.string() })
        return this.request(`/admin/api/v1/workflows/${uuid}`, ApiResponseSchema(Schema), {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async getWorkflow(uuid: string): Promise<{
        uuid: string
        name: string
        description?: string | null
        kind: 'consumer' | 'provider'
        enabled: boolean
        schedule_cron?: string | null
        config: unknown
    }> {
        const Schema = z.object({
            uuid: UuidSchema,
            name: z.string(),
            description: z.string().nullable().optional(),
            kind: z.enum(['consumer', 'provider']),
            enabled: z.boolean(),
            schedule_cron: z.string().nullable().optional(),
            config: z.any(),
        })
        return this.request(`/admin/api/v1/workflows/${uuid}`, ApiResponseSchema(Schema))
    }

    async previewCron(expr: string): Promise<string[]> {
        const Schema = z.array(z.string())
        return this.request(
            `/admin/api/v1/workflows/cron/preview?expr=${encodeURIComponent(expr)}`,
            ApiResponseSchema(Schema)
        )
    }

    async getWorkflowRuns(
        workflowUuid: string,
        page = 1,
        perPage = 20
    ): Promise<{
        data: Array<{
            uuid: string
            status: string
            queued_at?: string | null
            finished_at?: string | null
            processed_items?: number | null
            failed_items?: number | null
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
        }
    }> {
        const Schema = z.array(
            z.object({
                uuid: UuidSchema,
                status: z.string(),
                queued_at: z.string().nullable().optional(),
                finished_at: z.string().nullable().optional(),
                processed_items: z.number().nullable().optional(),
                failed_items: z.number().nullable().optional(),
            })
        )
        return this.paginatedRequest(
            `/admin/api/v1/workflows/${workflowUuid}/runs?page=${page}&per_page=${perPage}`,
            PaginatedApiResponseSchema(Schema)
        )
    }

    async getWorkflowRunLogs(
        runUuid: string,
        page = 1,
        perPage = 50
    ): Promise<{
        data: Array<{ uuid: string; ts: string; level: string; message: string; meta?: unknown }>
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
        }
    }> {
        const Schema = z.array(
            z.object({
                uuid: UuidSchema,
                ts: z.string(),
                level: z.string(),
                message: z.string(),
                meta: z.any().optional(),
            })
        )
        return this.paginatedRequest(
            `/admin/api/v1/workflows/runs/${runUuid}/logs?page=${page}&per_page=${perPage}`,
            PaginatedApiResponseSchema(Schema)
        )
    }

    async getAllWorkflowRuns(
        page = 1,
        perPage = 20
    ): Promise<{
        data: Array<{
            uuid: string
            status: string
            queued_at?: string | null
            finished_at?: string | null
            processed_items?: number | null
            failed_items?: number | null
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
        }
    }> {
        const Schema = z.array(
            z.object({
                uuid: UuidSchema,
                status: z.string(),
                queued_at: z.string().nullable().optional(),
                finished_at: z.string().nullable().optional(),
                processed_items: z.number().nullable().optional(),
                failed_items: z.number().nullable().optional(),
            })
        )
        return this.paginatedRequest(
            `/admin/api/v1/workflows/runs?page=${page}&per_page=${perPage}`,
            PaginatedApiResponseSchema(Schema)
        )
    }

    async getWorkflows(
        page = 1,
        itemsPerPage = 20
    ): Promise<{
        data: Array<{
            uuid: string
            name: string
            kind: 'consumer' | 'provider'
            enabled: boolean
            schedule_cron?: string | null
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
        }

    }> {
        const Schema = z.array(
            z.object({
                uuid: UuidSchema,
                name: z.string(),
                kind: z.enum(['consumer', 'provider']),
                enabled: z.boolean(),
                schedule_cron: z.string().nullable().optional(),
            })
        )
        return this.paginatedRequest(
            `/admin/api/v1/workflows?page=${page}&per_page=${itemsPerPage}`,
            PaginatedApiResponseSchema(Schema)
        )
    }

  // DSL endpoints (delegated)
  async getDslFromOptions() {
    const { getDslFromOptions } = await import('./clients/dsl')
    return getDslFromOptions(this)
  }
  async getDslToOptions() {
    const { getDslToOptions } = await import('./clients/dsl')
    return getDslToOptions(this)
  }
  async getDslTransformOptions() {
    const { getDslTransformOptions } = await import('./clients/dsl')
    return getDslTransformOptions(this)
  }
  async validateDsl(steps: unknown[]) {
    const { validateDsl } = await import('./clients/dsl')
    return validateDsl(this, steps)
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

    async uploadRunFile(
        workflowUuid: string,
        file: File
    ): Promise<{ run_uuid: string; staged_items: number }> {
        const form = new FormData()
        form.append('file', file)
        const schema = z.object({
            run_uuid: UuidSchema,
            staged_items: z.number(),
        })
        // Bypass JSON content-type; handle raw fetch here due to multipart
        const authStore = useAuthStore()
        const res = await fetch(
            `${this.baseURL}/admin/api/v1/workflows/${workflowUuid}/run/upload`,
            {
                method: 'POST',
                headers: {
                    ...(authStore.token && { Authorization: `Bearer ${authStore.token}` }),
                },
                body: form,
            }
        )
        if (!res.ok) {
            // Try to extract standardized error
            try {
                const err = await res.json()
                if (err?.message) {
                    throw new Error(err.message)
                }
            } catch {
                // fallthrough
            }
            throw new Error(`HTTP ${res.status}: ${res.statusText}`)
        }
        const json = await res.json()
        const parsed = ApiResponseSchema(schema).parse(json)

        return parsed.data
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
                        entity_uuid: UuidSchema.nullable().optional(),
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

    // New: entity fields helper for DSL mapping
    async getEntityFields(entityType: string): Promise<Array<{ name: string; type: string; required: boolean; system: boolean }>> {
        const Schema = z.array(
            z.object({
                name: z.string(),
                type: z.string(),
                required: z.boolean(),
                system: z.boolean(),
            })
        )
        return this.request(
            `/admin/api/v1/entity-definitions/${encodeURIComponent(entityType)}/fields`,
            ApiResponseSchema(Schema)
        )
    }
}

export const typedHttpClient = new TypedHttpClient()
export type { TypedHttpClient }
