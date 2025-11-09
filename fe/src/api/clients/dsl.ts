import { z } from 'zod'
import { ApiResponseSchema } from '@/types/schemas/base'
import { DslOptionsResponseSchema, DslValidateRequestSchema } from '@/types/schemas/dsl'

export async function getDslFromOptions(client: any) {
    return client.request('/admin/api/v1/dsl/from/options', ApiResponseSchema(DslOptionsResponseSchema))
}

export async function getDslToOptions(client: any) {
    return client.request('/admin/api/v1/dsl/to/options', ApiResponseSchema(DslOptionsResponseSchema))
}

export async function getDslTransformOptions(client: any) {
    return client.request('/admin/api/v1/dsl/transform/options', ApiResponseSchema(DslOptionsResponseSchema))
}

export async function validateDsl(client: any, steps: unknown[]) {
    const request = { steps }
    const ValidateRespSchema = z.object({ valid: z.boolean() })
    return client.request('/admin/api/v1/dsl/validate', ApiResponseSchema(ValidateRespSchema), {
        method: 'POST',
        body: JSON.stringify(request),
    })
}


