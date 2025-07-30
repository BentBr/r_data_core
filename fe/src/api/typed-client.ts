import { z } from 'zod'
import { env } from '@/env-check'
import {
    ApiResponseSchema,
    ClassDefinitionSchema,
    ApiKeySchema,
    UserSchema,
    LoginResponseSchema,
    type ApiResponse,
    type ClassDefinition,
    type ApiKey,
    type User,
    type LoginResponse,
    type LoginRequest,
} from '@/types/schemas'

class TypedHttpClient {
    private baseURL = env.apiBaseUrl
    private enableLogging = env.enableApiLogging
    private devMode = env.devMode

    async request<T>(
        endpoint: string,
        schema: z.ZodType<ApiResponse<T>>,
        options: RequestInit = {},
    ): Promise<T> {
        // TODO: Add auth token when auth store is ready
        const config: RequestInit = {
            ...options,
            headers: {
                'Content-Type': 'application/json',
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
                    // TODO: Handle logout when auth store is ready
                    throw new Error('Authentication required')
                }
                throw new Error(`HTTP ${response.status}: ${response.statusText}`)
            }

            const rawData = await response.json()

            if (this.enableLogging && this.devMode) {
                console.log(`[API] Response:`, rawData)
            }

            // Runtime validation with Zod
            const validatedResponse = schema.parse(rawData)

            if (validatedResponse.status === 'Error') {
                throw new Error(validatedResponse.message)
            }

            if (!validatedResponse.data) {
                throw new Error('No data in successful response')
            }

            return validatedResponse.data
        } catch (error) {
            if (error instanceof z.ZodError) {
                if (this.enableLogging) {
                    console.error('[API] Response validation failed:', error.issues)
                }
                throw new Error(`Invalid response format: ${error.issues[0]?.message}`)
            }
            if (this.enableLogging) {
                console.error('[API] Error:', error)
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
            ApiResponseSchema(z.array(ClassDefinitionSchema)),
        )
    }

    async getClassDefinition(uuid: string): Promise<ClassDefinition> {
        return this.request(
            `/admin/api/v1/class-definitions/${uuid}`,
            ApiResponseSchema(ClassDefinitionSchema),
        )
    }

    async createClassDefinition(data: Partial<ClassDefinition>): Promise<{ uuid: string }> {
        return this.request(
            '/admin/api/v1/class-definitions',
            ApiResponseSchema(z.object({ uuid: z.string().uuid() })),
            {
                method: 'POST',
                body: JSON.stringify(data),
            },
        )
    }

    async updateClassDefinition(
        uuid: string,
        data: Partial<ClassDefinition>,
    ): Promise<{ uuid: string }> {
        return this.request(
            `/admin/api/v1/class-definitions/${uuid}`,
            ApiResponseSchema(z.object({ uuid: z.string().uuid() })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            },
        )
    }

    async deleteClassDefinition(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/class-definitions/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            },
        )
    }

    async getApiKeys(limit?: number, offset = 0): Promise<ApiKey[]> {
        const pageSize = limit ?? this.getDefaultPageSize()
        return this.request(
            `/admin/api/v1/api-keys?limit=${pageSize}&offset=${offset}`,
            ApiResponseSchema(z.array(ApiKeySchema)),
        )
    }

    async login(credentials: LoginRequest): Promise<LoginResponse> {
        return this.request(
            '/admin/api/v1/auth/login',
            ApiResponseSchema(LoginResponseSchema),
            {
                method: 'POST',
                body: JSON.stringify(credentials),
            },
        )
    }

    async getUsers(limit?: number, offset = 0): Promise<User[]> {
        const pageSize = limit ?? this.getDefaultPageSize()
        return this.request(
            `/admin/api/v1/users?limit=${pageSize}&offset=${offset}`,
            ApiResponseSchema(z.array(UserSchema)),
        )
    }
}

export const typedHttpClient = new TypedHttpClient()
export type { TypedHttpClient } 