import { z } from 'zod'
import {
    ApiResponseSchema,
    PaginatedApiResponseSchema,
    DynamicEntitySchema,
    EntityResponseSchema,
    UuidSchema,
} from '@/types/schemas'
import type {
    DynamicEntity,
    CreateEntityRequest,
    UpdateEntityRequest,
    EntityResponse,
    ApiResponse,
} from '@/types/schemas'
import { BaseTypedHttpClient } from './base'

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
        const response = await this.paginatedRequest(
            `/api/v1/${entityType}?page=${page}&per_page=${itemsPerPage}${includeParam}`,
            PaginatedApiResponseSchema(z.array(DynamicEntitySchema))
        )
        return response
    }

    async browseByPath(
        path: string,
        limit = this.getDefaultPageSize(),
        offset = 0
    ): Promise<{
        data: Array<{
            kind: 'folder' | 'file'
            name: string
            path: string
            entity_uuid?: string
            entity_type?: string
            has_children?: boolean
        }>
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
        const encoded = encodeURIComponent(path)
        const response = await this.paginatedRequest(
            `/api/v1/entities/by-path?path=${encoded}&limit=${limit}&offset=${offset}`,
            PaginatedApiResponseSchema(
                z.array(
                    z.object({
                        kind: z.enum(['folder', 'file']),
                        name: z.string(),
                        path: z.string(),
                        entity_uuid: UuidSchema.nullable().optional(),
                        entity_type: z.string().nullable().optional(),
                        has_children: z.boolean().nullable().optional(),
                    })
                )
            )
        )
        return response
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
        return this.request(
            '/api/v1/entities/query',
            ApiResponseSchema(z.array(DynamicEntitySchema)),
            {
                method: 'POST',
                body: JSON.stringify({
                    entity_type: entityType,
                    parent_uuid: parentUuid,
                    path: path,
                    limit: limit,
                    offset: offset,
                }),
            }
        )
    }

    async getEntity(entityType: string, uuid: string): Promise<DynamicEntity> {
        return this.request(`/api/v1/${entityType}/${uuid}`, ApiResponseSchema(DynamicEntitySchema))
    }

    async createEntity(
        entityType: string,
        data: CreateEntityRequest
    ): Promise<EntityResponse | null> {
        const body: Record<string, unknown> = { ...data.data }
        if (data.parent_uuid !== undefined && data.parent_uuid !== null) {
            body.parent_uuid = data.parent_uuid
        }
        return this.request(
            `/api/v1/${entityType}`,
            ApiResponseSchema(EntityResponseSchema) as z.ZodType<
                ApiResponse<EntityResponse | null>
            >,
            {
                method: 'POST',
                body: JSON.stringify(body),
            }
        )
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
        return this.request(
            `/api/v1/${entityType}/${uuid}`,
            ApiResponseSchema(EntityResponseSchema) as z.ZodType<
                ApiResponse<EntityResponse | null>
            >,
            {
                method: 'PUT',
                body: JSON.stringify(body),
            }
        )
    }

    async deleteEntity(entityType: string, uuid: string): Promise<{ message: string }> {
        return this.request(
            `/api/v1/${entityType}/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async listEntityVersions(entityType: string, uuid: string): Promise<Array<{
        version_number: number
        created_at: string
        created_by?: string | null
        created_by_name?: string | null
    }>> {
        return this.request(
            `/api/v1/entities/${encodeURIComponent(entityType)}/${uuid}/versions`,
            ApiResponseSchema(
                z.array(
                    z.object({
                        version_number: z.number(),
                        created_at: z.string(),
                        created_by: UuidSchema.nullable().optional(),
                        created_by_name: z.string().nullable().optional(),
                    })
                )
            )
        )
    }

    async getEntityVersion(entityType: string, uuid: string, versionNumber: number): Promise<{
        version_number: number
        created_at: string
        created_by?: string | null
        data: Record<string, unknown>
    }> {
        return this.request(
            `/api/v1/entities/${encodeURIComponent(entityType)}/${uuid}/versions/${versionNumber}`,
            ApiResponseSchema(
                z.object({
                    version_number: z.number(),
                    created_at: z.string(),
                    created_by: UuidSchema.nullable().optional(),
                    data: z.any(), // Use z.any() instead of z.record(z.any()) to handle nested objects
                })
            )
        )
    }
}
