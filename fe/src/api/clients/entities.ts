import type {
    CreateEntityRequest,
    DynamicEntity,
    EntityResponse,
    UpdateEntityRequest,
    ResponseMeta,
} from '@/types/schemas'
import type { BrowseNode } from '@/types/generated/BrowseNode'
import type { VersionMeta } from '@/types/generated/VersionMeta'
import type { VersionPayload } from '@/types/generated/VersionPayload'
import { BaseTypedHttpClient } from './base'

type PaginatedBrowseResult = { data: BrowseNode[]; meta?: ResponseMeta }

export class EntitiesClient extends BaseTypedHttpClient {
    async getEntities(
        entityType: string,
        page = 1,
        itemsPerPage = 10,
        include?: string
    ): Promise<{ data: DynamicEntity[]; meta?: ResponseMeta }> {
        const includeParam = include ? `&include=${include}` : ''
        return this.paginatedRequest<DynamicEntity[]>(
            `/api/v1/${entityType}?page=${page}&per_page=${itemsPerPage}${includeParam}`
        )
    }

    async browseByPath(
        path: string,
        limit = this.getDefaultPageSize(),
        offset = 0
    ): Promise<PaginatedBrowseResult> {
        const encoded = encodeURIComponent(path)
        return this.paginatedRequest<BrowseNode[]>(
            `/api/v1/entities/by-path?path=${encoded}&limit=${limit}&offset=${offset}`
        )
    }

    async searchEntitiesByPath(searchTerm: string, limit = 10): Promise<PaginatedBrowseResult> {
        const encoded = encodeURIComponent(searchTerm)
        return this.paginatedRequest<BrowseNode[]>(
            `/api/v1/entities/by-path?search=${encoded}&limit=${limit}`
        )
    }

    async queryEntities(
        entityType: string,
        options: {
            parentUuid?: string
            path?: string
            limit?: number
            offset?: number
        }
    ): Promise<DynamicEntity[]> {
        const { parentUuid, path, limit = 20, offset = 0 } = options
        return this.request<DynamicEntity[]>('/api/v1/entities/query', {
            method: 'POST',
            body: JSON.stringify({
                entity_type: entityType,
                parent_uuid: parentUuid,
                path: path,
                limit: limit,
                offset: offset,
            }),
        })
    }

    async getEntity(
        entityType: string,
        uuid: string,
        options?: { includeChildrenCount?: boolean }
    ): Promise<DynamicEntity> {
        const params = new URLSearchParams()
        if (options?.includeChildrenCount) {
            params.set('include_children_count', 'true')
        }
        const queryString = params.toString()
        const url = queryString
            ? `/api/v1/${entityType}/${uuid}?${queryString}`
            : `/api/v1/${entityType}/${uuid}`
        return this.request<DynamicEntity>(url)
    }

    async createEntity(
        entityType: string,
        data: CreateEntityRequest
    ): Promise<EntityResponse | null> {
        const body: Record<string, unknown> = { ...data.data }
        if (data.parent_uuid !== undefined && data.parent_uuid !== null) {
            body.parent_uuid = data.parent_uuid
        }
        return this.request<EntityResponse | null>(`/api/v1/${entityType}`, {
            method: 'POST',
            body: JSON.stringify(body),
        })
    }

    async updateEntity(
        entityType: string,
        uuid: string,
        data: UpdateEntityRequest
    ): Promise<EntityResponse | null> {
        const body: Record<string, unknown> = { ...data.data }
        if (data.parent_uuid !== undefined) {
            body.parent_uuid = data.parent_uuid
        }
        return this.request<EntityResponse | null>(`/api/v1/${entityType}/${uuid}`, {
            method: 'PUT',
            body: JSON.stringify(body),
        })
    }

    async deleteEntity(entityType: string, uuid: string): Promise<{ message: string }> {
        return this.request<{ message: string }>(`/api/v1/${entityType}/${uuid}`, {
            method: 'DELETE',
        })
    }

    async listEntityVersions(entityType: string, uuid: string): Promise<VersionMeta[]> {
        return this.request<VersionMeta[]>(
            `/api/v1/entities/${encodeURIComponent(entityType)}/${uuid}/versions`
        )
    }

    async getEntityVersion(
        entityType: string,
        uuid: string,
        versionNumber: number
    ): Promise<VersionPayload> {
        return this.request<VersionPayload>(
            `/api/v1/entities/${encodeURIComponent(entityType)}/${uuid}/versions/${versionNumber}`
        )
    }
}
