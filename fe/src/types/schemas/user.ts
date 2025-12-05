import { z } from 'zod'
import { UuidSchema, TimestampSchema, NullableUuidSchema } from './base'

// Email validation helper
const emailValidation = z
    .string()
    .min(1, 'Email is required')
    .refine(val => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(val), 'Invalid email format')

// User schema (for login response)
export const UserSchema = z.object({
    uuid: UuidSchema,
    username: z.string(),
    email: emailValidation,
    first_name: z.string(),
    last_name: z.string(),
    role_uuids: z.array(UuidSchema),
    is_active: z.boolean(),
    is_admin: z.boolean(),
    created_at: TimestampSchema,
    updated_at: TimestampSchema,
})

// User response schema (for admin user management)
export const UserResponseSchema = z.object({
    uuid: UuidSchema,
    username: z.string(),
    email: emailValidation,
    full_name: z.string(),
    first_name: z.string().nullable(),
    last_name: z.string().nullable(),
    role_uuids: z.array(UuidSchema),
    status: z.string(),
    is_active: z.boolean(),
    is_admin: z.boolean(),
    super_admin: z.boolean(),
    last_login: TimestampSchema.nullable(),
    failed_login_attempts: z.number(),
    created_at: TimestampSchema,
    updated_at: TimestampSchema,
    created_by: NullableUuidSchema,
})

// Create user request schema
export const CreateUserRequestSchema = z.object({
    username: z.string().min(3).max(50),
    email: emailValidation,
    password: z.string().min(8),
    first_name: z.string(),
    last_name: z.string(),
    role_uuids: z.array(UuidSchema).optional(),
    is_active: z.boolean().optional(),
    super_admin: z.boolean().optional(),
})

// Update user request schema
export const UpdateUserRequestSchema = z.object({
    email: emailValidation.optional(),
    password: z.string().min(8).optional(),
    first_name: z.string().optional(),
    last_name: z.string().optional(),
    role_uuids: z.array(UuidSchema).optional(),
    is_active: z.boolean().optional(),
    super_admin: z.boolean().optional(),
})

// Type exports
export type User = z.infer<typeof UserSchema>
export type UserResponse = z.infer<typeof UserResponseSchema>
export type CreateUserRequest = z.infer<typeof CreateUserRequestSchema>
export type UpdateUserRequest = z.infer<typeof UpdateUserRequestSchema>
