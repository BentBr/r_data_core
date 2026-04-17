import { z } from 'zod'
import { UuidSchema } from './base'
import { API_KEY_NAME_MIN_LENGTH } from '../generated/validation'

// Create API key request schema (form validation)
// Note: satisfies z.ZodType<GeneratedCreateApiKeyRequest> not applied because the generated
// type uses `bigint | null` for expires_in_days whereas Zod uses `number` (JS has no bigint in Zod).
export const CreateApiKeyRequestSchema = z.object({
    name: z.string().min(API_KEY_NAME_MIN_LENGTH),
    description: z.string().optional(),
    expires_in_days: z.number().int().positive().optional(),
})

export const ReassignApiKeyRequestSchema = z.object({
    user_uuid: UuidSchema,
})

export const ReassignApiKeyResponseSchema = z.object({
    message: z.string(),
})

// Type exports — re-exported from generated for consumers that only need types
export type { ApiKeyResponse as ApiKey } from '../generated/ApiKeyResponse'
export type { ApiKeyCreatedResponse } from '../generated/ApiKeyCreatedResponse'
export type CreateApiKeyRequest = z.infer<typeof CreateApiKeyRequestSchema>
export type ReassignApiKeyRequest = z.infer<typeof ReassignApiKeyRequestSchema>
export type ReassignApiKeyResponse = z.infer<typeof ReassignApiKeyResponseSchema>

// Custom data type for API key meta
export type ApiKeyCustomData = Record<string, unknown>
