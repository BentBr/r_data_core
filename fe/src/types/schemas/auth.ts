import { z } from 'zod'
import type { AdminLoginRequest } from '../generated/AdminLoginRequest'
import { USERNAME_MIN_LENGTH, PASSWORD_MIN_LENGTH } from '../generated/validation'

// Auth schemas (form validation — kept as Zod for runtime validation)
export const LoginRequestSchema = z.object({
    username: z.string().min(USERNAME_MIN_LENGTH),
    password: z.string().min(PASSWORD_MIN_LENGTH),
}) satisfies z.ZodType<AdminLoginRequest>

export const RefreshTokenRequestSchema = z.object({
    refresh_token: z.string(),
})

export const LogoutRequestSchema = z.object({
    refresh_token: z.string(),
})

// Type exports — response types re-exported from generated for consumers that only need types
export type LoginRequest = z.infer<typeof LoginRequestSchema>
export type { AdminLoginResponse as LoginResponse } from '../generated/AdminLoginResponse'
export type { RefreshTokenResponse } from '../generated/RefreshTokenResponse'
export type RefreshTokenRequest = z.infer<typeof RefreshTokenRequestSchema>
export type LogoutRequest = z.infer<typeof LogoutRequestSchema>
