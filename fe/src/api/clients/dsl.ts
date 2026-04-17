import { BaseTypedHttpClient } from './base'
import type { DslStep, DslOptionsResponse } from '@/types/schemas'

export class DslClient extends BaseTypedHttpClient {
    async getDslFromOptions(): Promise<DslOptionsResponse> {
        return this.request<DslOptionsResponse>('/admin/api/v1/dsl/from/options')
    }

    async getDslToOptions(): Promise<DslOptionsResponse> {
        return this.request<DslOptionsResponse>('/admin/api/v1/dsl/to/options')
    }

    async getDslTransformOptions(): Promise<DslOptionsResponse> {
        return this.request<DslOptionsResponse>('/admin/api/v1/dsl/transform/options')
    }

    async validateDsl(steps: DslStep[]): Promise<{ valid: boolean }> {
        const request = { steps }
        return this.request<{ valid: boolean }>('/admin/api/v1/dsl/validate', {
            method: 'POST',
            body: JSON.stringify(request),
        })
    }
}
