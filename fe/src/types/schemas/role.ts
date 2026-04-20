import { z } from 'zod'
import { UuidSchema } from './base'
import type { PermissionType as GeneratedPermissionType } from '../generated/PermissionType'
import type { AccessLevel as GeneratedAccessLevel } from '../generated/AccessLevel'
import type { ResourceNamespace as GeneratedResourceNamespace } from '../generated/ResourceNamespace'
import type { PermissionResponse } from '../generated/PermissionResponse'
import type { CreateRoleRequest as GeneratedCreateRoleRequest } from '../generated/CreateRoleRequest'
import type { UpdateRoleRequest as GeneratedUpdateRoleRequest } from '../generated/UpdateRoleRequest'
import type { AssignRolesRequest as GeneratedAssignRolesRequest } from '../generated/AssignRolesRequest'

// Enums — guarded against drift from generated unions via `satisfies`
export const ResourceNamespaceSchema = z.enum([
    'Workflows',
    'Entities',
    'EntityDefinitions',
    'ApiKeys',
    'Roles',
    'Users',
    'System',
    'DashboardStats',
]) satisfies z.ZodType<GeneratedResourceNamespace>

export const PermissionTypeSchema = z.enum([
    'Read',
    'Create',
    'Update',
    'Delete',
    'Publish',
    'Admin',
    'Execute',
]) satisfies z.ZodType<GeneratedPermissionType>

export const AccessLevelSchema = z.enum([
    'None',
    'Own',
    'Group',
    'All',
]) satisfies z.ZodType<GeneratedAccessLevel>

// Permission schema — binds to generated PermissionResponse (constraints: unknown)
export const PermissionSchema = z.object({
    resource_type: z.string(),
    permission_type: PermissionTypeSchema,
    access_level: AccessLevelSchema,
    resource_uuids: z.array(UuidSchema),
    constraints: z.unknown(),
}) satisfies z.ZodType<PermissionResponse>

// Request schemas (form validation) — bind to generated types
export const CreateRoleRequestSchema = z.object({
    name: z.string(),
    description: z.string().optional(),
    super_admin: z.boolean().optional(),
    permissions: z.array(PermissionSchema),
}) satisfies z.ZodType<GeneratedCreateRoleRequest>

export const UpdateRoleRequestSchema = z.object({
    name: z.string(),
    description: z.string().optional(),
    super_admin: z.boolean().optional(),
    permissions: z.array(PermissionSchema),
}) satisfies z.ZodType<GeneratedUpdateRoleRequest>

export const AssignRolesRequestSchema = z.object({
    role_uuids: z.array(UuidSchema),
}) satisfies z.ZodType<GeneratedAssignRolesRequest>

// Type exports
export type ResourceNamespace = z.infer<typeof ResourceNamespaceSchema>
export type PermissionType = z.infer<typeof PermissionTypeSchema>
export type AccessLevel = z.infer<typeof AccessLevelSchema>
export type Permission = z.infer<typeof PermissionSchema>
// Role type re-exported from generated
export type { RoleResponse as Role } from '../generated/RoleResponse'
export type CreateRoleRequest = z.infer<typeof CreateRoleRequestSchema>
export type UpdateRoleRequest = z.infer<typeof UpdateRoleRequestSchema>
export type AssignRolesRequest = z.infer<typeof AssignRolesRequestSchema>

/**
 * Role custom data/metadata
 * Flexible type for storing custom key-value pairs with roles
 */
export interface RoleCustomData extends Record<string, unknown> {
    [key: string]: unknown
}
