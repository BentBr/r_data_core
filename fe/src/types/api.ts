// API Response types matching the backend format
export interface ApiResponse<T> {
    status: 'Success' | 'Error'
    message: string
    data?: T
    meta?: {
        pagination?: {
            total: number
            page: number
            per_page: number
            total_pages: number
            has_previous: boolean
            has_next: boolean
        }
        request_id: string
        timestamp: string
        custom?: Record<string, unknown>
    }
}

// Auth related types
export interface LoginRequest {
    username: string
    password: string
}

export interface LoginResponse {
    token: string
    user_uuid: string
    username: string
    role: string
    expires_at: string
}

// Entity Definition types
export interface EntityDefinition {
    uuid: string
    entity_type: string
    display_name: string
    description?: string
    field_definitions: FieldDefinition[]
    created_at: string
    updated_at: string
    created_by: string
    updated_by?: string
    version: number
}

export interface FieldDefinition {
    field_name: string
    display_name: string
    field_type: string
    is_required: boolean
    constraints?: Record<string, unknown>
    ui_options?: Record<string, unknown>
}

// Entity types
export interface DynamicEntity {
    uuid: string
    entity_type: string
    data: Record<string, unknown>
    created_at: string
    updated_at: string
}

// API Key types
export interface ApiKey {
    uuid: string
    name: string
    description?: string
    is_active: boolean
    created_at: string
    expires_at?: string
    last_used_at?: string
    created_by: string
    user_uuid: string
    published: boolean
}

export interface CreateApiKeyRequest {
    name: string
    description?: string
    expires_in_days?: number
}

export interface ApiKeyCreatedResponse extends ApiKey {
    api_key: string
}

// User types
export interface User {
    uuid: string
    username: string
    email: string
    first_name: string
    last_name: string
    role: string
    is_active: boolean
    created_at: string
    updated_at: string
}
