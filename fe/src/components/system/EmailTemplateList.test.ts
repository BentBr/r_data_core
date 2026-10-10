import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest'
import { mount } from '@vue/test-utils'
import EmailTemplateList from './EmailTemplateList.vue'
import type { EmailTemplate } from '@/api/clients/email-templates'

const mockListEmailTemplates: Mock = vi.fn()
const mockDeleteEmailTemplate: Mock = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        listEmailTemplates: (...args: unknown[]) => mockListEmailTemplates(...args),
        deleteEmailTemplate: (uuid: string) => mockDeleteEmailTemplate(uuid),
    },
}))

const showSuccess = vi.fn()
vi.mock('@/composables/useSnackbar', () => ({
    useSnackbar: () => ({
        currentSnackbar: null,
        showSuccess,
    }),
}))

const mockHandleError = vi.fn()
vi.mock('@/composables/useErrorHandler', () => ({
    useErrorHandler: () => ({
        handleError: mockHandleError,
    }),
}))

const makeTemplate = (overrides: Partial<EmailTemplate> = {}): EmailTemplate => ({
    uuid: 'tmpl-1',
    name: 'Welcome Email',
    slug: 'welcome_email',
    template_type: 'workflow',
    subject_template: 'Hello {{name}}',
    body_html_template: '<p>Hi {{name}}</p>',
    body_text_template: 'Hi {{name}}',
    variables: [],
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-02T00:00:00Z',
    ...overrides,
})

const flush = async () => {
    await Promise.resolve()
    await Promise.resolve()
}

describe('EmailTemplateList', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockListEmailTemplates.mockResolvedValue([])
        mockDeleteEmailTemplate.mockResolvedValue(undefined)
    })

    it('loads templates on mount', async () => {
        const template = makeTemplate()
        mockListEmailTemplates.mockResolvedValue([template])

        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()

        expect(mockListEmailTemplates).toHaveBeenCalled()
        expect((wrapper.vm as any).templates).toEqual([template])
        expect((wrapper.vm as any).loading).toBe(false)
    })

    it('routes load errors through the error handler and stops loading', async () => {
        mockListEmailTemplates.mockRejectedValue(new Error('boom'))

        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()

        expect(mockHandleError).toHaveBeenCalled()
        expect((wrapper.vm as any).loading).toBe(false)
    })

    it('opens the editor in create mode', async () => {
        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()

        ;(wrapper.vm as any).openCreate()

        expect((wrapper.vm as any).showEditor).toBe(true)
        expect((wrapper.vm as any).editingTemplate).toBeNull()
    })

    it('opens the editor in edit mode with the selected template', async () => {
        const template = makeTemplate()
        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()

        ;(wrapper.vm as any).openEdit(template)

        expect((wrapper.vm as any).showEditor).toBe(true)
        expect((wrapper.vm as any).editingTemplate).toEqual(template)
    })

    it('opens the delete confirmation dialog for a template', async () => {
        const template = makeTemplate()
        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()

        ;(wrapper.vm as any).confirmDelete(template)

        expect((wrapper.vm as any).showDeleteDialog).toBe(true)
        expect((wrapper.vm as any).templateToDelete).toEqual(template)
    })

    it('deletes the template, shows success, and reloads the list', async () => {
        const template = makeTemplate()
        mockListEmailTemplates.mockResolvedValue([template])
        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()
        ;(wrapper.vm as any).confirmDelete(template)
        mockListEmailTemplates.mockClear()

        await (wrapper.vm as any).performDelete()

        expect(mockDeleteEmailTemplate).toHaveBeenCalledWith('tmpl-1')
        expect(showSuccess).toHaveBeenCalled()
        expect((wrapper.vm as any).showDeleteDialog).toBe(false)
        expect((wrapper.vm as any).templateToDelete).toBeNull()
        expect(mockListEmailTemplates).toHaveBeenCalled()
    })

    it('does nothing when deleting without a selected template', async () => {
        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()

        await (wrapper.vm as any).performDelete()

        expect(mockDeleteEmailTemplate).not.toHaveBeenCalled()
    })

    it('routes delete errors through the error handler', async () => {
        mockDeleteEmailTemplate.mockRejectedValue(new Error('delete failed'))
        const template = makeTemplate()
        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()
        ;(wrapper.vm as any).confirmDelete(template)

        await (wrapper.vm as any).performDelete()

        expect(mockHandleError).toHaveBeenCalled()
        expect((wrapper.vm as any).deleting).toBe(false)
    })

    it('formats a valid date string', async () => {
        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()

        const formatted = (wrapper.vm as any).formatDate('2024-01-02T00:00:00Z')
        expect(formatted).not.toBe('2024-01-02T00:00:00Z')
        expect(typeof formatted).toBe('string')
    })

    it('reloads the list when the editor emits saved', async () => {
        mockListEmailTemplates.mockResolvedValue([])
        const wrapper = mount(EmailTemplateList, {
            global: { stubs: { EmailTemplateEditor: true } },
        })
        await flush()
        mockListEmailTemplates.mockClear()

        await wrapper.findComponent({ name: 'EmailTemplateEditor' }).vm.$emit('saved')
        await flush()

        expect(mockListEmailTemplates).toHaveBeenCalled()
    })
})
