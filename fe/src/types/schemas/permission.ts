import { z } from 'zod'
import { UuidSchema } from './base'

// Enums
export const ResourceNamespaceSchema = z.enum([
    'Workflows',
    'Entities',
    'EntityDefinitions',
    'ApiKeys',
    'Roles',
    'Users',
    'System',
])

export const PermissionTypeSchema = z.enum([
    'Read',
    'Create',
    'Update',
    'Delete',
    'Publish',
    'Admin',
    'Execute',
])

export const AccessLevelSchema = z.enum(['None', 'Own', 'Group', 'All'])

// Permission schema
export const PermissionSchema = z.object({
    resource_type: z.string(), // ResourceNamespace as string
    permission_type: PermissionTypeSchema,
    access_level: AccessLevelSchema,
    resource_uuids: z.array(UuidSchema),
    constraints: z.record(z.string(), z.unknown()).nullish(),
})

// Request schemas
export const CreateRoleRequestSchema = z.object({
    name: z.string(),
    description: z.string().nullable().optional(),
    super_admin: z.boolean().optional(),
    permissions: z.array(PermissionSchema),
})

export const UpdateRoleRequestSchema = z.object({
    name: z.string(),
    description: z.string().nullable().optional(),
    super_admin: z.boolean().optional(),
    permissions: z.array(PermissionSchema),
})

export const AssignRolesRequestSchema = z.object({
    role_uuids: z.array(UuidSchema),
})

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
