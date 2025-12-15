import { z } from 'zod'
import { ApiResponseSchema } from '@/types/schemas/base'
import { DslOptionsResponseSchema } from '@/types/schemas/dsl'
import { BaseTypedHttpClient } from './base'
import type { DslStep, DslOptionsResponse } from '@/types/schemas'

export class DslClient extends BaseTypedHttpClient {
    async getDslFromOptions(): Promise<DslOptionsResponse> {
        return this.request(
            '/admin/api/v1/dsl/from/options',
            ApiResponseSchema(DslOptionsResponseSchema)
        )
    }

    async getDslToOptions(): Promise<DslOptionsResponse> {
        return this.request(
            '/admin/api/v1/dsl/to/options',
            ApiResponseSchema(DslOptionsResponseSchema)
        )
    }

    async getDslTransformOptions(): Promise<DslOptionsResponse> {
        return this.request(
            '/admin/api/v1/dsl/transform/options',
            ApiResponseSchema(DslOptionsResponseSchema)
        )
    }

    async validateDsl(steps: DslStep[]): Promise<{ valid: boolean }> {
        const request = { steps }
        const ValidateRespSchema = z.object({ valid: z.boolean() })
        return this.request('/admin/api/v1/dsl/validate', ApiResponseSchema(ValidateRespSchema), {
            method: 'POST',
            body: JSON.stringify(request),
        })
    }
}
