import { BaseTypedHttpClient } from './base'

export interface EntityVersioningSettings {
    enabled: boolean
    max_versions?: number | null
    max_age_days?: number | null
}

export interface WorkflowRunLogSettings {
    enabled: boolean
    max_runs?: number | null
    max_age_days?: number | null
}

export type LicenseState = 'none' | 'invalid' | 'error' | 'valid'

export interface LicenseStatus {
    state: LicenseState
    company?: string | null
    license_type?: string | null
    license_id?: string | null
    issued_at?: string | null
    expires_at?: string | null
    version?: string | null
    verified_at: string
    error_message?: string | null
}

export interface ComponentVersion {
    name: string
    version: string
    last_seen_at: string
}

export interface SystemVersions {
    core: string
    worker?: ComponentVersion | null
    maintenance?: ComponentVersion | null
}

export class SystemClient extends BaseTypedHttpClient {
    async getEntityVersioningSettings(): Promise<EntityVersioningSettings> {
        return this.request<EntityVersioningSettings>(
            '/admin/api/v1/system/settings/entity-versioning'
        )
    }

    async updateEntityVersioningSettings(
        payload: EntityVersioningSettings
    ): Promise<EntityVersioningSettings> {
        return this.request<EntityVersioningSettings>(
            '/admin/api/v1/system/settings/entity-versioning',
            {
                method: 'PUT',
                body: JSON.stringify(payload),
            }
        )
    }

    async getWorkflowRunLogSettings(): Promise<WorkflowRunLogSettings> {
        return this.request<WorkflowRunLogSettings>(
            '/admin/api/v1/system/settings/workflow-run-logs'
        )
    }

    async updateWorkflowRunLogSettings(
        payload: WorkflowRunLogSettings
    ): Promise<WorkflowRunLogSettings> {
        return this.request<WorkflowRunLogSettings>(
            '/admin/api/v1/system/settings/workflow-run-logs',
            {
                method: 'PUT',
                body: JSON.stringify(payload),
            }
        )
    }

    async getLicenseStatus(): Promise<LicenseStatus> {
        return this.request<LicenseStatus>('/admin/api/v1/system/license')
    }

    async getSystemVersions(): Promise<SystemVersions> {
        return this.request<SystemVersions>('/admin/api/v1/system/versions')
    }
}
