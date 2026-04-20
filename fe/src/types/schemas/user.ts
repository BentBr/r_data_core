import { z } from 'zod'
import { UuidSchema } from './base'
import type { CreateUserRequest as GeneratedCreateUserRequest } from '../generated/CreateUserRequest'
import type { UpdateUserRequest as GeneratedUpdateUserRequest } from '../generated/UpdateUserRequest'
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
export const CreateUserRequestSchema = z.object({
    username: z.string().min(USERNAME_MIN_LENGTH).max(USERNAME_MAX_LENGTH),
    email: emailValidation,
    password: z.string().min(PASSWORD_MIN_LENGTH),
    first_name: z.string(),
    last_name: z.string(),
    role_uuids: z.array(UuidSchema).nullable(),
    is_active: z.boolean().nullable(),
    super_admin: z.boolean().nullable(),
}) satisfies z.ZodType<GeneratedCreateUserRequest>

// Update user request schema (form validation)
export const UpdateUserRequestSchema = z.object({
    email: emailValidation.nullable(),
    password: z.string().min(PASSWORD_MIN_LENGTH).nullable(),
    first_name: z.string().nullable(),
    last_name: z.string().nullable(),
    role_uuids: z.array(UuidSchema).nullable(),
    is_active: z.boolean().nullable(),
    super_admin: z.boolean().nullable(),
}) satisfies z.ZodType<GeneratedUpdateUserRequest>

// Type exports — re-exported from generated for consumers that only need types
export type { UserResponse } from '../generated/UserResponse'
export type CreateUserRequest = z.infer<typeof CreateUserRequestSchema>
export type UpdateUserRequest = z.infer<typeof UpdateUserRequestSchema>

/**
 * User custom data/metadata
 * Flexible type for storing custom key-value pairs with users
 */
export interface UserCustomData extends Record<string, unknown> {
    [key: string]: unknown
}
