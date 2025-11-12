// Main export file for all schemas
// Re-export everything from submodules for backward compatibility

export * from './base'
export * from './common'
export * from './auth'
export * from './entity'
export * from './workflow'
export * from './api-key'
export * from './user'
export * from './dsl'

// Re-export types that are commonly used
import type { z } from 'zod'
import { ApiResponseSchema, MetaSchema, PaginationSchema } from './base'
import type {
    TableAction,
    TableColumn,
    TreeNode,
    SnackbarConfig,
    DialogConfig,
    FormField,
} from './common'
import type {
    LoginRequest,
    LoginResponse,
    RefreshTokenRequest,
    RefreshTokenResponse,
    LogoutRequest,
} from './auth'
import type {
    FieldDefinition,
    EntityDefinition,
    DynamicEntity,
    CreateEntityRequest,
    UpdateEntityRequest,
    EntityResponse,
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
} from './entity'
import type { Workflow, WorkflowRun, WorkflowRunLog } from './workflow'
import type {
    ApiKey,
    CreateApiKeyRequest,
    ApiKeyCreatedResponse,
    ReassignApiKeyRequest,
    ReassignApiKeyResponse,
} from './api-key'
import type { User } from './user'

// Common type exports
export type Pagination = z.infer<typeof PaginationSchema>
export type Meta = z.infer<typeof MetaSchema>
export type ApiResponse<T> = z.infer<ReturnType<typeof ApiResponseSchema<z.ZodType<T>>>>

// Re-export all types
export type {
    TableAction,
    TableColumn,
    TreeNode,
    SnackbarConfig,
    DialogConfig,
    FormField,
    LoginRequest,
    LoginResponse,
    RefreshTokenRequest,
    RefreshTokenResponse,
    LogoutRequest,
    FieldDefinition,
    EntityDefinition,
    DynamicEntity,
    CreateEntityRequest,
    UpdateEntityRequest,
    EntityResponse,
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
    Workflow,
    WorkflowRun,
    WorkflowRunLog,
    ApiKey,
    CreateApiKeyRequest,
    ApiKeyCreatedResponse,
    ReassignApiKeyRequest,
    ReassignApiKeyResponse,
    User,
}
