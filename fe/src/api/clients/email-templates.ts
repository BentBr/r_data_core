import { BaseTypedHttpClient } from './base'
import type { EmailTemplateResponse } from '@/types/generated/EmailTemplateResponse'
import type { CreateEmailTemplateRequest } from '@/types/generated/CreateEmailTemplateRequest'
import type { UpdateEmailTemplateRequest } from '@/types/generated/UpdateEmailTemplateRequest'
import type { EmailTemplateType } from '@/types/generated/EmailTemplateType'

export type EmailTemplate = EmailTemplateResponse
export type { CreateEmailTemplateRequest, UpdateEmailTemplateRequest }

export class EmailTemplateClient extends BaseTypedHttpClient {
    async list(type?: EmailTemplateType): Promise<EmailTemplate[]> {
        const params = type ? `?type=${type}` : ''
        return this.request<EmailTemplate[]>(`/admin/api/v1/email-templates${params}`)
    }

    async getByUuid(uuid: string): Promise<EmailTemplate> {
        return this.request<EmailTemplate>(`/admin/api/v1/email-templates/${uuid}`)
    }

    async create(data: CreateEmailTemplateRequest): Promise<{ uuid: string }> {
        return this.request<{ uuid: string }>('/admin/api/v1/email-templates', {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    async update(uuid: string, data: UpdateEmailTemplateRequest): Promise<void> {
        return this.request<void>(`/admin/api/v1/email-templates/${uuid}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    async delete(uuid: string): Promise<void> {
        return this.request<void>(`/admin/api/v1/email-templates/${uuid}`, {
            method: 'DELETE',
        })
    }
}
