import { z } from 'zod'
import { UuidSchema, TimestampSchema, NullableTimestampSchema } from './base'

// API Key schema
export const ApiKeySchema = z.object({
    uuid: UuidSchema,
    name: z.string(),
    description: z.string().optional(),
    is_active: z.boolean(),
    created_at: TimestampSchema,
    expires_at: NullableTimestampSchema,
    last_used_at: NullableTimestampSchema,
    created_by: UuidSchema,
    user_uuid: UuidSchema,
    published: z.boolean(),
})

export const CreateApiKeyRequestSchema = z.object({
    name: z.string().min(1),
    description: z.string().optional(),
    expires_in_days: z.number().int().positive().optional(),
})

export const ApiKeyCreatedResponseSchema = ApiKeySchema.extend({
    api_key: z.string(),
})

export const ReassignApiKeyRequestSchema = z.object({
    user_uuid: UuidSchema,
})

export const ReassignApiKeyResponseSchema = z.object({
    message: z.string(),
})

// Type exports
export type ApiKey = z.infer<typeof ApiKeySchema>
export type CreateApiKeyRequest = z.infer<typeof CreateApiKeyRequestSchema>
export type ApiKeyCreatedResponse = z.infer<typeof ApiKeyCreatedResponseSchema>
export type ReassignApiKeyRequest = z.infer<typeof ReassignApiKeyRequestSchema>
export type ReassignApiKeyResponse = z.infer<typeof ReassignApiKeyResponseSchema>

