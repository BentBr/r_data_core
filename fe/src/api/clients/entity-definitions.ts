import { z } from 'zod'
import {
    ApiResponseSchema,
    PaginatedApiResponseSchema,
    EntityDefinitionSchema,
    UuidSchema,
} from '@/types/schemas'
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
        const response = await this.paginatedRequest(
            `/admin/api/v1/entity-definitions?limit=${pageSize}&offset=${offset}`,
            PaginatedApiResponseSchema(z.array(EntityDefinitionSchema))
        )
        return response
    }

    async getEntityDefinition(uuid: string): Promise<EntityDefinition> {
        return this.request(
            `/admin/api/v1/entity-definitions/${uuid}`,
            ApiResponseSchema(EntityDefinitionSchema)
        )
    }

    async createEntityDefinition(data: CreateEntityDefinitionRequest): Promise<{ uuid: string }> {
        return this.request(
            '/admin/api/v1/entity-definitions',
            ApiResponseSchema(z.object({ uuid: UuidSchema })),
            {
                method: 'POST',
                body: JSON.stringify(data),
            }
        )
    }

    async updateEntityDefinition(
        uuid: string,
        data: UpdateEntityDefinitionRequest
    ): Promise<{ uuid: string }> {
        return this.request(
            `/admin/api/v1/entity-definitions/${uuid}`,
            ApiResponseSchema(z.object({ uuid: UuidSchema })),
            {
                method: 'PUT',
                body: JSON.stringify(data),
            }
        )
    }

    async deleteEntityDefinition(uuid: string): Promise<{ message: string }> {
        return this.request(
            `/admin/api/v1/entity-definitions/${uuid}`,
            ApiResponseSchema(z.object({ message: z.string() })),
            {
                method: 'DELETE',
            }
        )
    }

    async applyEntityDefinitionSchema(uuid?: string): Promise<{ message: string }> {
        const endpoint = '/admin/api/v1/entity-definitions/apply-schema'
        return this.request(endpoint, ApiResponseSchema(z.object({ message: z.string() })), {
            method: 'POST',
            body: JSON.stringify({ uuid }),
        })
    }

    async getEntityFields(
        entityType: string
    ): Promise<Array<{ name: string; type: string; required: boolean; system: boolean }>> {
        const Schema = z.array(
            z.object({
                name: z.string(),
                type: z.string(),
                required: z.boolean(),
                system: z.boolean(),
            })
        )
        return this.request(
            `/admin/api/v1/entity-definitions/${encodeURIComponent(entityType)}/fields`,
            ApiResponseSchema(Schema)
        )
    }

    async listEntityDefinitionVersions(uuid: string): Promise<Array<{
        version_number: number
        created_at: string
        created_by?: string | null
        created_by_name?: string | null
    }>> {
        return this.request(
            `/admin/api/v1/entity-definitions/${uuid}/versions`,
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

    async getEntityDefinitionVersion(uuid: string, versionNumber: number): Promise<{
        version_number: number
        created_at: string
        created_by?: string | null
        data: Record<string, unknown>
    }> {
        return this.request(
            `/admin/api/v1/entity-definitions/${uuid}/versions/${versionNumber}`,
            ApiResponseSchema(
                z.object({
                    version_number: z.number(),
                    created_at: z.string(),
                    created_by: UuidSchema.nullable().optional(),
                    data: z.any(),
                })
            )
        )
    }
}
