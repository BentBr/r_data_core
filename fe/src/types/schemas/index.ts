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
export * from './role'

// Re-export types that are commonly used
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
import type { Status } from '../generated/Status'
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
import type { Workflow, WorkflowRun, WorkflowRunLog, WorkflowConfig } from './workflow'
import type {
    ApiKey,
    CreateApiKeyRequest,
    ApiKeyCreatedResponse,
    ReassignApiKeyRequest,
    ReassignApiKeyResponse,
    ApiKeyCustomData,
} from './api-key'
import type { UserCustomData } from './user'
import type { UserResponse } from '../generated/UserResponse'
import type {
    Role,
    Permission,
    CreateRoleRequest,
    UpdateRoleRequest,
    AssignRolesRequest,
    ResourceNamespace,
    PermissionType,
    AccessLevel,
    RoleCustomData,
} from './role'

// Re-export all types
export type {
    TableAction,
    TableColumn,
    TreeNode,
    SnackbarConfig,
    DialogConfig,
    FormField,
    Status,
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
    WorkflowConfig,
    ApiKey,
    CreateApiKeyRequest,
    ApiKeyCreatedResponse,
    ReassignApiKeyRequest,
    ReassignApiKeyResponse,
    ApiKeyCustomData,
    UserResponse,
    UserCustomData,
    Role,
    Permission,
    CreateRoleRequest,
    UpdateRoleRequest,
    AssignRolesRequest,
    ResourceNamespace,
    PermissionType,
    AccessLevel,
    RoleCustomData,
}
