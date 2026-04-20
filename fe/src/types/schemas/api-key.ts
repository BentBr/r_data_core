import { z } from 'zod'
import { UuidSchema } from './base'
import type { CreateApiKeyRequest as GeneratedCreateApiKeyRequest } from '../generated/CreateApiKeyRequest'
import type { ReassignApiKeyRequest as GeneratedReassignApiKeyRequest } from '../generated/ReassignApiKeyRequest'
import { API_KEY_NAME_MIN_LENGTH } from '../generated/validation'

// Create API key request schema (form validation)
export const CreateApiKeyRequestSchema = z.object({
    name: z.string().min(API_KEY_NAME_MIN_LENGTH),
    description: z.string().optional(),
    expires_in_days: z.number().int().positive().optional(),
}) satisfies z.ZodType<GeneratedCreateApiKeyRequest>

export const ReassignApiKeyRequestSchema = z.object({
    user_uuid: UuidSchema,
}) satisfies z.ZodType<GeneratedReassignApiKeyRequest>

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
