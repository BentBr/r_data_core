import { BaseTypedHttpClient } from './base'

export interface SystemLog {
    uuid: string
    created_at: string
    created_by: string | null
    status: 'success' | 'failed' | 'pending'
    log_type: 'email_sent' | 'entity_created' | 'entity_updated' | 'entity_deleted' | 'auth_event'
    resource_type:
        | 'email'
        | 'admin_user'
        | 'role'
        | 'workflow'
        | 'entity_definition'
        | 'email_template'
    resource_uuid: string | null
    summary: string
    details: unknown | null
}

export interface SystemLogListResponse {
    data: SystemLog[]
    total: number
}

export class SystemLogClient extends BaseTypedHttpClient {
    async list(params: {
        page?: number
        page_size?: number
        log_type?: string
        resource_type?: string
        status?: string
    }): Promise<SystemLogListResponse> {
        const query = new URLSearchParams()
        if (params.page) query.set('page', String(params.page))
        if (params.page_size) query.set('page_size', String(params.page_size))
        if (params.log_type) query.set('log_type', params.log_type)
        if (params.resource_type) query.set('resource_type', params.resource_type)
        if (params.status) query.set('status', params.status)
        const qs = query.toString()
        return this.request<SystemLogListResponse>(`/admin/api/v1/system/logs${qs ? `?${qs}` : ''}`)
    }

    async getByUuid(uuid: string): Promise<SystemLog> {
        return this.request<SystemLog>(`/admin/api/v1/system/logs/${uuid}`)
    }
}
