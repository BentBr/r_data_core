import type {
    EntityDefinition,
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
    ResponseMeta,
} from '@/types/schemas'
import type { ApplySchemaRequest } from '@/types/generated/ApplySchemaRequest'
import type { EntityDefinitionVersionMeta } from '@/types/generated/EntityDefinitionVersionMeta'
import type { EntityDefinitionVersionPayload } from '@/types/generated/EntityDefinitionVersionPayload'
import type { EntityFieldInfo } from '@/types/generated/EntityFieldInfo'
import type { PaginationQuery } from '@/types/generated/PaginationQuery'
import { BaseTypedHttpClient } from './base'
import { buildListQueryString } from './query'

export class EntityDefinitionsClient extends BaseTypedHttpClient {
    async getEntityDefinitions(
        pagination: PaginationQuery
    ): Promise<{ data: EntityDefinition[]; meta?: ResponseMeta }> {
        return this.paginatedRequest<EntityDefinition[]>(
            `/admin/api/v1/entity-definitions${buildListQueryString(pagination)}`
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
        const body: ApplySchemaRequest = { uuid: uuid ?? null }
        return this.request<{ message: string }>(endpoint, {
            method: 'POST',
            body: JSON.stringify(body),
        })
    }

    async getEntityFields(entityType: string): Promise<EntityFieldInfo[]> {
        return this.request<EntityFieldInfo[]>(
            `/admin/api/v1/entity-definitions/${encodeURIComponent(entityType)}/fields`
        )
    }

    async listEntityDefinitionVersions(uuid: string): Promise<EntityDefinitionVersionMeta[]> {
        return this.request<EntityDefinitionVersionMeta[]>(
            `/admin/api/v1/entity-definitions/${uuid}/versions`
        )
    }

    async getEntityDefinitionVersion(
        uuid: string,
        versionNumber: number
    ): Promise<EntityDefinitionVersionPayload> {
        return this.request<EntityDefinitionVersionPayload>(
            `/admin/api/v1/entity-definitions/${uuid}/versions/${versionNumber}`
        )
    }
}
