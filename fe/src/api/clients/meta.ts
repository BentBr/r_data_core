import { BaseTypedHttpClient } from './base'

interface EntityTypeCount {
    entity_type: string
    count: number
}

interface EntityStats {
    total: number
    by_type: EntityTypeCount[]
}

interface WorkflowWithLatestStatus {
    uuid: string
    name: string
    latest_status?: string | null
}

interface WorkflowStats {
    total: number
    workflows: WorkflowWithLatestStatus[]
}

export interface DashboardStats {
    entity_definitions_count: number
    entities: EntityStats
    workflows: WorkflowStats
    online_users_count: number
}

export type { EntityStats, EntityTypeCount, WorkflowStats, WorkflowWithLatestStatus }

export class MetaClient extends BaseTypedHttpClient {
    async getDashboardStats(): Promise<DashboardStats> {
        return this.request<DashboardStats>('/admin/api/v1/meta/dashboard')
    }
}
