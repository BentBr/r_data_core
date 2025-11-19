import { z } from 'zod'
import { ApiResponseSchema } from '@/types/schemas/base'
import { DslOptionsResponseSchema } from '@/types/schemas/dsl'
import type { BaseTypedHttpClient } from './base'
import type { DslStep } from '@/types/schemas'

export async function getDslFromOptions(client: BaseTypedHttpClient) {
    return client.request(
        '/admin/api/v1/dsl/from/options',
        ApiResponseSchema(DslOptionsResponseSchema)
    )
}

export async function getDslToOptions(client: BaseTypedHttpClient) {
    return client.request(
        '/admin/api/v1/dsl/to/options',
        ApiResponseSchema(DslOptionsResponseSchema)
    )
}

export async function getDslTransformOptions(client: BaseTypedHttpClient) {
    return client.request(
        '/admin/api/v1/dsl/transform/options',
        ApiResponseSchema(DslOptionsResponseSchema)
    )
}

export async function validateDsl(client: BaseTypedHttpClient, steps: DslStep[]) {
    const request = { steps }
    const ValidateRespSchema = z.object({ valid: z.boolean() })
    return client.request('/admin/api/v1/dsl/validate', ApiResponseSchema(ValidateRespSchema), {
        method: 'POST',
        body: JSON.stringify(request),
    })
}
