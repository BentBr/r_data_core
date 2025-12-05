import { z } from 'zod'
import { ApiResponseSchema } from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

const EntityTypeCountSchema = z.object({
    entity_type: z.string(),
    count: z.number(),
})

const EntityStatsSchema = z.object({
    total: z.number(),
    by_type: z.array(EntityTypeCountSchema),
})

const WorkflowWithLatestStatusSchema = z.object({
    uuid: z.string(),
    name: z.string(),
    latest_status: z.string().nullable().optional(),
})

const WorkflowStatsSchema = z.object({
    total: z.number(),
    workflows: z.array(WorkflowWithLatestStatusSchema),
})

const DashboardStatsSchema = z.object({
    entity_definitions_count: z.number(),
    entities: EntityStatsSchema,
    workflows: WorkflowStatsSchema,
    online_users_count: z.number(),
})

export type DashboardStats = z.infer<typeof DashboardStatsSchema>
export type EntityStats = z.infer<typeof EntityStatsSchema>
export type EntityTypeCount = z.infer<typeof EntityTypeCountSchema>
export type WorkflowStats = z.infer<typeof WorkflowStatsSchema>
export type WorkflowWithLatestStatus = z.infer<typeof WorkflowWithLatestStatusSchema>

export class MetaClient extends BaseTypedHttpClient {
    async getDashboardStats(): Promise<DashboardStats> {
        return this.request('/admin/api/v1/meta/dashboard', ApiResponseSchema(DashboardStatsSchema))
    }
}
