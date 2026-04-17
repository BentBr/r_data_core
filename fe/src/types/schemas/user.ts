import { z } from 'zod'
import { UuidSchema } from './base'
import {
    EMAIL_PATTERN,
    USERNAME_MIN_LENGTH,
    USERNAME_MAX_LENGTH,
    PASSWORD_MIN_LENGTH,
} from '../generated/validation'

// Email validation helper (uses constant from generated/validation)
const emailValidation = z
    .string()
    .min(1, 'Email is required')
    .refine(val => EMAIL_PATTERN.test(val), 'Invalid email format')

// Create user request schema (form validation)
// Note: satisfies z.ZodType<GeneratedCreateUserRequest> not applied because the generated
// type uses `string[] | null` for optional fields whereas Zod uses `.optional()` —
// the Rust-side serialisation sends null for absent fields; the FE omits them entirely.
export const CreateUserRequestSchema = z.object({
    username: z.string().min(USERNAME_MIN_LENGTH).max(USERNAME_MAX_LENGTH),
    email: emailValidation,
    password: z.string().min(PASSWORD_MIN_LENGTH),
    first_name: z.string(),
    last_name: z.string(),
    role_uuids: z.array(UuidSchema).optional(),
    is_active: z.boolean().optional(),
    super_admin: z.boolean().optional(),
})

// Update user request schema (form validation)
export const UpdateUserRequestSchema = z.object({
    email: emailValidation.optional(),
    password: z.string().min(PASSWORD_MIN_LENGTH).optional(),
    first_name: z.string().optional(),
    last_name: z.string().optional(),
    role_uuids: z.array(UuidSchema).optional(),
    is_active: z.boolean().optional(),
    super_admin: z.boolean().optional(),
})

// Type exports — re-exported from generated for consumers that only need types
export type { UserResponse } from '../generated/UserResponse'
export type CreateUserRequest = z.infer<typeof CreateUserRequestSchema>
export type UpdateUserRequest = z.infer<typeof UpdateUserRequestSchema>

// Legacy User type — FE-only shape used in auth store
export interface User {
    uuid: string
    username: string
    email: string
    first_name: string
    last_name: string
    role_uuids: string[]
    is_active: boolean
    is_admin: boolean
    created_at: string
    updated_at: string
}

/**
 * User custom data/metadata
 * Flexible type for storing custom key-value pairs with users
 */
export interface UserCustomData extends Record<string, unknown> {
    [key: string]: unknown
}
