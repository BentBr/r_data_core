import { z } from 'zod'
import { BaseTypedHttpClient } from './base'
import { ApiResponseSchema } from '@/types/schemas'

export const EntityVersioningSettingsSchema = z.object({
    enabled: z.boolean(),
    max_versions: z.number().nullable().optional(),
    max_age_days: z.number().nullable().optional(),
})
export type EntityVersioningSettings = z.infer<typeof EntityVersioningSettingsSchema>

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
}
