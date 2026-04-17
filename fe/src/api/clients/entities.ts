import type {
    CreateEntityRequest,
    DynamicEntity,
    EntityResponse,
    UpdateEntityRequest,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

type PathEntry = {
    kind: 'folder' | 'file'
    name: string
    path: string
    entity_uuid?: string | null
    entity_type?: string | null
    has_children?: boolean | null
    published: boolean
}

type PaginatedPathResult = {
    data: PathEntry[]
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
}

export class EntitiesClient extends BaseTypedHttpClient {
    async getEntities(
        entityType: string,
        page = 1,
        itemsPerPage = 10,
        include?: string
    ): Promise<{
        data: DynamicEntity[]
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
        const includeParam = include ? `&include=${include}` : ''
        return this.paginatedRequest<DynamicEntity[]>(
            `/api/v1/${entityType}?page=${page}&per_page=${itemsPerPage}${includeParam}`
        )
    }

    async browseByPath(
        path: string,
        limit = this.getDefaultPageSize(),
        offset = 0
    ): Promise<PaginatedPathResult> {
        const encoded = encodeURIComponent(path)
        return this.paginatedRequest<PathEntry[]>(
            `/api/v1/entities/by-path?path=${encoded}&limit=${limit}&offset=${offset}`
        )
    }

    async searchEntitiesByPath(searchTerm: string, limit = 10): Promise<PaginatedPathResult> {
        const encoded = encodeURIComponent(searchTerm)
        return this.paginatedRequest<PathEntry[]>(
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

    async listEntityVersions(
        entityType: string,
        uuid: string
    ): Promise<
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
        >(`/api/v1/entities/${encodeURIComponent(entityType)}/${uuid}/versions`)
    }

    async getEntityVersion(
        entityType: string,
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
        }>(`/api/v1/entities/${encodeURIComponent(entityType)}/${uuid}/versions/${versionNumber}`)
    }
}
