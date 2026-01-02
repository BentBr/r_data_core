import { z } from 'zod'
import { BaseTypedHttpClient } from './base'
import { ApiResponseSchema } from '@/types/schemas'

export const EntityVersioningSettingsSchema = z.object({
    enabled: z.boolean(),
    max_versions: z.number().nullable().optional(),
    max_age_days: z.number().nullable().optional(),
})
export type EntityVersioningSettings = z.infer<typeof EntityVersioningSettingsSchema>

export const LicenseStateSchema = z.enum(['none', 'invalid', 'error', 'valid'])
export type LicenseState = z.infer<typeof LicenseStateSchema>

export const LicenseStatusSchema = z.object({
    state: LicenseStateSchema,
    company: z.string().nullable().optional(),
    license_type: z.string().nullable().optional(),
    license_id: z.string().nullable().optional(),
    issued_at: z.string().nullable().optional(),
    expires_at: z.string().nullable().optional(),
    version: z.string().nullable().optional(),
    verified_at: z.string(),
    error_message: z.string().nullable().optional(),
})
export type LicenseStatus = z.infer<typeof LicenseStatusSchema>

export class SystemClient extends BaseTypedHttpClient {
    async getEntityVersioningSettings(): Promise<EntityVersioningSettings> {
        return this.request(
            '/admin/api/v1/system/settings/entity-versioning',
            ApiResponseSchema(EntityVersioningSettingsSchema)
        )
    }

    async updateEntityVersioningSettings(
        payload: EntityVersioningSettings
    ): Promise<EntityVersioningSettings> {
        return this.request(
            '/admin/api/v1/system/settings/entity-versioning',
            ApiResponseSchema(EntityVersioningSettingsSchema),
            {
                method: 'PUT',
                body: JSON.stringify(payload),
            }
        )
    }

    async getLicenseStatus(): Promise<LicenseStatus> {
        return this.request('/admin/api/v1/system/license', ApiResponseSchema(LicenseStatusSchema))
    }
}
