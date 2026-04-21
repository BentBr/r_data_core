import type { DashboardStats } from '@/types/generated/DashboardStats'
import { BaseTypedHttpClient } from './base'

export type { DashboardStats } from '@/types/generated/DashboardStats'
export type { EntityStats } from '@/types/generated/EntityStats'
export type { EntityTypeCount } from '@/types/generated/EntityTypeCount'
export type { WorkflowStats } from '@/types/generated/WorkflowStats'
export type { WorkflowWithLatestStatus } from '@/types/generated/WorkflowWithLatestStatus'

export class MetaClient extends BaseTypedHttpClient {
    async getDashboardStats(): Promise<DashboardStats> {
        return this.request<DashboardStats>('/admin/api/v1/meta/dashboard')
    }
}
