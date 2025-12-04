import { z } from 'zod'
import { UuidSchema } from './base'

// Enums
export const ResourceNamespaceSchema = z.enum([
    'Workflows',
    'Entities',
    'EntityDefinitions',
    'ApiKeys',
    'PermissionSchemes',
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
    'Custom',
])

export const AccessLevelSchema = z.enum(['None', 'Own', 'Group', 'All'])

// Permission schema
export const PermissionSchema = z.object({
    resource_type: z.string(), // ResourceNamespace as string
    permission_type: PermissionTypeSchema,
    access_level: AccessLevelSchema,
    resource_uuids: z.array(UuidSchema),
    constraints: z.record(z.unknown()).optional(),
})

// Permission Scheme schema
export const PermissionSchemeSchema = z.object({
    uuid: UuidSchema,
    name: z.string(),
    description: z.string().nullable(),
    is_system: z.boolean(),
    super_admin: z.boolean(),
    role_permissions: z.record(z.array(PermissionSchema)),
    created_at: z.string(),
    updated_at: z.string(),
    created_by: UuidSchema,
    updated_by: UuidSchema.nullable(),
    published: z.boolean(),
    version: z.number(),
})

// Request/Response schemas
export const CreatePermissionSchemeRequestSchema = z.object({
    name: z.string(),
    description: z.string().nullable().optional(),
    super_admin: z.boolean().optional(),
    role_permissions: z.record(z.array(PermissionSchema)),
})

export const UpdatePermissionSchemeRequestSchema = z.object({
    name: z.string(),
    description: z.string().nullable().optional(),
    super_admin: z.boolean().optional(),
    role_permissions: z.record(z.array(PermissionSchema)),
})

export const AssignSchemesRequestSchema = z.object({
    scheme_uuids: z.array(UuidSchema),
})

// Type exports
export type ResourceNamespace = z.infer<typeof ResourceNamespaceSchema>
export type PermissionType = z.infer<typeof PermissionTypeSchema>
export type AccessLevel = z.infer<typeof AccessLevelSchema>
export type Permission = z.infer<typeof PermissionSchema>
export type PermissionScheme = z.infer<typeof PermissionSchemeSchema>
export type CreatePermissionSchemeRequest = z.infer<typeof CreatePermissionSchemeRequestSchema>
export type UpdatePermissionSchemeRequest = z.infer<typeof UpdatePermissionSchemeRequestSchema>
export type AssignSchemesRequest = z.infer<typeof AssignSchemesRequestSchema>
