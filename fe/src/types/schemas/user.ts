import { z } from 'zod'
import { UuidSchema, TimestampSchema } from './base'

// User schema
export const UserSchema = z.object({
    uuid: UuidSchema,
    username: z.string(),
    email: z.string().email(),
    first_name: z.string(),
    last_name: z.string(),
    role: z.string(),
    is_active: z.boolean(),
    is_admin: z.boolean(),
    created_at: TimestampSchema,
    updated_at: TimestampSchema,
})

// Type exports
export type User = z.infer<typeof UserSchema>
