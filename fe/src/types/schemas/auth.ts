import { z } from 'zod'
import { UuidSchema, TimestampSchema } from './base'

// Auth schemas
export const LoginRequestSchema = z.object({
    username: z.string().min(3),
    password: z.string().min(8),
})

export const LoginResponseSchema = z.object({
    access_token: z.string(),
    refresh_token: z.string(),
    user_uuid: UuidSchema,
    username: z.string(),
    access_expires_at: TimestampSchema,
    refresh_expires_at: TimestampSchema,
})

// Refresh token schemas
export const RefreshTokenRequestSchema = z.object({
    refresh_token: z.string(),
})

export const RefreshTokenResponseSchema = z.object({
    access_token: z.string(),
    refresh_token: z.string(),
    access_expires_at: TimestampSchema,
    refresh_expires_at: TimestampSchema,
})

export const LogoutRequestSchema = z.object({
    refresh_token: z.string(),
})

// Type exports
export type LoginRequest = z.infer<typeof LoginRequestSchema>
export type LoginResponse = z.infer<typeof LoginResponseSchema>
export type RefreshTokenRequest = z.infer<typeof RefreshTokenRequestSchema>
export type RefreshTokenResponse = z.infer<typeof RefreshTokenResponseSchema>
export type LogoutRequest = z.infer<typeof LogoutRequestSchema>
