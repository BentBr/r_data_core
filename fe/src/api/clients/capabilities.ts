import { BaseTypedHttpClient } from './base'

export interface CapabilitiesResponse {
    system_mail_configured: boolean
    workflow_mail_configured: boolean
}

export class CapabilitiesClient extends BaseTypedHttpClient {
    async getCapabilities(): Promise<CapabilitiesResponse> {
        return this.request<CapabilitiesResponse>('/admin/api/v1/system/capabilities')
    }
}
