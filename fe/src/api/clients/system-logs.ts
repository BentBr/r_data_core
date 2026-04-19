import { BaseTypedHttpClient } from './base'
import type { SystemLogDto } from '@/types/generated/SystemLogDto'
import type { SystemLogType } from '@/types/generated/SystemLogType'
import type { SystemLogResourceType } from '@/types/generated/SystemLogResourceType'
import type { SystemLogStatus } from '@/types/generated/SystemLogStatus'

export type SystemLog = SystemLogDto

export interface SystemLogListResult {
    data: SystemLog[]
    meta?: {
        pagination?: {
            total: number
            page: number
            per_page: number
            total_pages: number
            has_previous: boolean
            has_next: boolean
        }
    }
}

export class SystemLogClient extends BaseTypedHttpClient {
    async list(params: {
        page?: number
        page_size?: number
        log_type?: SystemLogType
        resource_type?: SystemLogResourceType
        status?: SystemLogStatus
        resource_uuid?: string
        date_from?: string
        date_to?: string
    }): Promise<SystemLogListResult> {
        const query = new URLSearchParams()
        if (params.page) query.set('page', String(params.page))
        if (params.page_size) query.set('page_size', String(params.page_size))
        if (params.log_type) query.set('log_type', params.log_type)
        if (params.resource_type) query.set('resource_type', params.resource_type)
        if (params.status) query.set('status', params.status)
        if (params.resource_uuid) query.set('resource_uuid', params.resource_uuid)
        if (params.date_from) query.set('date_from', params.date_from)
        if (params.date_to) query.set('date_to', params.date_to)
        const qs = query.toString()
        return this.paginatedRequest<SystemLog[]>(`/admin/api/v1/system/logs${qs ? `?${qs}` : ''}`)
    }

    async getByUuid(uuid: string): Promise<SystemLog> {
        return this.request<SystemLog>(`/admin/api/v1/system/logs/${uuid}`)
    }
}
