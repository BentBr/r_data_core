import type { EntityVersioningSettingsDto } from '@/types/generated/EntityVersioningSettingsDto'
import type { WorkflowRunLogSettingsDto } from '@/types/generated/WorkflowRunLogSettingsDto'
import type { UpdateSettingsBody } from '@/types/generated/UpdateSettingsBody'
import type { UpdateWorkflowRunLogSettingsBody } from '@/types/generated/UpdateWorkflowRunLogSettingsBody'
import type { LicenseStatusDto } from '@/types/generated/LicenseStatusDto'
import type { SystemVersionsDto } from '@/types/generated/SystemVersionsDto'
import { BaseTypedHttpClient } from './base'

// FE-facing aliases over BE-generated shapes; callers keep using short names without redeclaring.
export type EntityVersioningSettings = EntityVersioningSettingsDto
export type WorkflowRunLogSettings = WorkflowRunLogSettingsDto
export type LicenseStatus = LicenseStatusDto
export type SystemVersions = SystemVersionsDto
export type { LicenseStateDto as LicenseState } from '@/types/generated/LicenseStateDto'
export type { ComponentVersionDto as ComponentVersion } from '@/types/generated/ComponentVersionDto'

export class SystemClient extends BaseTypedHttpClient {
    async getEntityVersioningSettings(): Promise<EntityVersioningSettingsDto> {
        return this.request<EntityVersioningSettingsDto>(
            '/admin/api/v1/system/settings/entity-versioning'
        )
    }

    async updateEntityVersioningSettings(
        payload: UpdateSettingsBody
    ): Promise<EntityVersioningSettingsDto> {
        return this.request<EntityVersioningSettingsDto>(
            '/admin/api/v1/system/settings/entity-versioning',
            {
                method: 'PUT',
                body: JSON.stringify(payload),
            }
        )
    }

    async getWorkflowRunLogSettings(): Promise<WorkflowRunLogSettingsDto> {
        return this.request<WorkflowRunLogSettingsDto>(
            '/admin/api/v1/system/settings/workflow-run-logs'
        )
    }

    async updateWorkflowRunLogSettings(
        payload: UpdateWorkflowRunLogSettingsBody
    ): Promise<WorkflowRunLogSettingsDto> {
        return this.request<WorkflowRunLogSettingsDto>(
            '/admin/api/v1/system/settings/workflow-run-logs',
            {
                method: 'PUT',
                body: JSON.stringify(payload),
            }
        )
    }

    async getLicenseStatus(): Promise<LicenseStatusDto> {
        return this.request<LicenseStatusDto>('/admin/api/v1/system/license')
    }

    async getSystemVersions(): Promise<SystemVersionsDto> {
        return this.request<SystemVersionsDto>('/admin/api/v1/system/versions')
    }
}
