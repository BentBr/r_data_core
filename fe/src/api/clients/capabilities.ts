import { BaseTypedHttpClient } from './base'
import type { CapabilitiesResponse } from '@/types/generated/CapabilitiesResponse'

export type { CapabilitiesResponse }

export class CapabilitiesClient extends BaseTypedHttpClient {
    async getCapabilities(): Promise<CapabilitiesResponse> {
        return this.request<CapabilitiesResponse>('/admin/api/v1/system/capabilities')
    }
}
