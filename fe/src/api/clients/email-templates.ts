import { BaseTypedHttpClient } from './base'

export interface EmailTemplate {
    uuid: string
    name: string
    slug: string
    template_type: 'system' | 'workflow'
    subject_template: string
    body_html_template: string
    body_text_template: string
    variables: Array<{ key: string; description: string }>
    created_at: string
    updated_at: string
}

export interface CreateEmailTemplateRequest {
    name: string
    slug: string
    subject_template: string
    body_html_template: string
    body_text_template: string
    variables: Array<{ key: string; description: string }>
}

export interface UpdateEmailTemplateRequest {
    name?: string
    subject_template: string
    body_html_template: string
    body_text_template: string
    variables: Array<{ key: string; description: string }>
}

export class EmailTemplateClient extends BaseTypedHttpClient {
    async list(type?: 'system' | 'workflow'): Promise<EmailTemplate[]> {
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
