import type {
    EntityDefinition,
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

export class EntityDefinitionsClient extends BaseTypedHttpClient {
    async getEntityDefinitions(
        limit?: number,
        offset = 0
    ): Promise<{
        data: EntityDefinition[]
        meta?: {
            pagination?: {
                total: number
                page: number
                per_page: number
                total_pages: number
                has_previous: boolean
                has_next: boolean
            }
            request_id?: string
            timestamp?: string
            custom?: unknown
        }
    }> {
        const pageSize = limit ?? this.getDefaultPageSize()
        return this.paginatedRequest<EntityDefinition[]>(
            `/admin/api/v1/entity-definitions?limit=${pageSize}&offset=${offset}`
        )
    }

    async getEntityDefinition(uuid: string): Promise<EntityDefinition> {
        return this.request<EntityDefinition>(`/admin/api/v1/entity-definitions/${uuid}`)
    }

    async createEntityDefinition(data: CreateEntityDefinitionRequest): Promise<{ uuid: string }> {
        return this.request<{ uuid: string }>('/admin/api/v1/entity-definitions', {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async updateEntityDefinition(
        uuid: string,
        data: UpdateEntityDefinitionRequest
    ): Promise<{ uuid: string }> {
        return this.request<{ uuid: string }>(`/admin/api/v1/entity-definitions/${uuid}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async deleteEntityDefinition(uuid: string): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/admin/api/v1/entity-definitions/${uuid}`, {
            method: 'DELETE',
        })
    }

    async applyEntityDefinitionSchema(uuid?: string): Promise<{ message: string }> {
        const endpoint = '/admin/api/v1/entity-definitions/apply-schema'
        return this.request<{ message: string }>(endpoint, {
            method: 'POST',
            body: JSON.stringify({ uuid }),
        })
    }

    async getEntityFields(
        entityType: string
    ): Promise<Array<{ name: string; type: string; required: boolean; system: boolean }>> {
        return this.request<
            Array<{ name: string; type: string; required: boolean; system: boolean }>
        >(`/admin/api/v1/entity-definitions/${encodeURIComponent(entityType)}/fields`)
    }

    async listEntityDefinitionVersions(uuid: string): Promise<
        Array<{
            version_number: number
            created_at: string
            created_by?: string | null
            created_by_name?: string | null
        }>
    > {
        return this.request<
            Array<{
                version_number: number
                created_at: string
                created_by?: string | null
                created_by_name?: string | null
            }>
        >(`/admin/api/v1/entity-definitions/${uuid}/versions`)
    }

    async getEntityDefinitionVersion(
        uuid: string,
        versionNumber: number
    ): Promise<{
        version_number: number
        created_at: string
        created_by?: string | null
        data: Record<string, unknown>
    }> {
        return this.request<{
            version_number: number
            created_at: string
            created_by?: string | null
            data: Record<string, unknown>
        }>(`/admin/api/v1/entity-definitions/${uuid}/versions/${versionNumber}`)
    }
}
