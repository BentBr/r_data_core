import { describe, it, expect, vi, beforeEach, afterEach, type Mock } from 'vitest'
import { mount, DOMWrapper, type VueWrapper } from '@vue/test-utils'
import EmailTemplateEditor from './EmailTemplateEditor.vue'
import type { EmailTemplate } from '@/api/clients/email-templates'

const mockUpdateEmailTemplate: Mock = vi.fn()
const mockCreateEmailTemplate: Mock = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        updateEmailTemplate: (...args: unknown[]) => mockUpdateEmailTemplate(...args),
        createEmailTemplate: (...args: unknown[]) => mockCreateEmailTemplate(...args),
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
    variables: [{ key: 'name', description: 'Recipient name' }],
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-02T00:00:00Z',
    ...overrides,
})

// v-dialog content is teleported; the global test harness stubs teleport
// (which prevents the real slot content from rendering), so text/button
// assertions need the real teleport behaviour — and since that moves the
// dialog's DOM into document.body, finding it means querying the body and
// unmounting afterwards so dialogs don't leak across tests.
interface EditorProps {
    modelValue: boolean
    template: EmailTemplate | null
}

let mountedWrappers: VueWrapper[] = []
const mountEditor = (props: EditorProps) => {
    const wrapper = mount(EmailTemplateEditor, {
        props,
        global: { stubs: { teleport: false } },
    })
    mountedWrappers.push(wrapper)
    return wrapper
}
const body = () => new DOMWrapper(document.body)

describe('EmailTemplateEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockUpdateEmailTemplate.mockResolvedValue(undefined)
        mockCreateEmailTemplate.mockResolvedValue({ uuid: 'new-uuid' })
    })

    afterEach(() => {
        mountedWrappers.forEach(w => w.unmount())
        mountedWrappers = []
    })

    it('starts with a blank form when creating a new template', () => {
        const wrapper = mountEditor({ modelValue: true, template: null })

        expect((wrapper.vm as any).form.name).toBe('')
        expect((wrapper.vm as any).form.slug).toBe('')
    })

    it('populates the form from an existing template', () => {
        const template = makeTemplate()
        const wrapper = mountEditor({ modelValue: true, template })

        expect((wrapper.vm as any).form.name).toBe('Welcome Email')
        expect((wrapper.vm as any).form.slug).toBe('welcome_email')
        expect((wrapper.vm as any).variableDescriptions).toEqual({ name: 'Recipient name' })
    })

    it('resets the form back to blank when the template prop becomes null', async () => {
        const template = makeTemplate()
        const wrapper = mountEditor({ modelValue: true, template })

        await wrapper.setProps({ template: null })

        expect((wrapper.vm as any).form.name).toBe('')
        expect((wrapper.vm as any).variableDescriptions).toEqual({})
    })

    it('auto-generates a slug from the name while creating', async () => {
        const wrapper = mountEditor({ modelValue: true, template: null })

        ;(wrapper.vm as any).form.name = 'My New Template!'
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).form.slug).toBe('my_new_template')
    })

    it('does not change the slug from the name when editing an existing template', async () => {
        const template = makeTemplate()
        const wrapper = mountEditor({ modelValue: true, template })

        ;(wrapper.vm as any).form.name = 'Renamed Template'
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).form.slug).toBe('welcome_email')
    })

    it('detects variable keys from subject and body templates, ignoring block keywords', () => {
        const wrapper = mountEditor({
            modelValue: true,
            template: makeTemplate({
                subject_template: 'Hi {{name}}',
                body_html_template: '{{#if this}}{{else}}{{order_id}}{{/if}}',
                body_text_template: '{{name}} {{amount}}',
            }),
        })

        expect((wrapper.vm as any).detectedVariableKeys).toEqual(['amount', 'name', 'order_id'])
    })

    it('shows no detected variables when none are present', () => {
        const wrapper = mountEditor({
            modelValue: true,
            template: makeTemplate({
                subject_template: 'Hello',
                body_html_template: '<p>Static</p>',
                body_text_template: 'Static',
            }),
        })

        expect((wrapper.vm as any).detectedVariableKeys).toEqual([])
        expect(body().text()).toContain('no_variables_detected')
    })

    it('wraps a variable key in handlebars braces for display', () => {
        const wrapper = mountEditor({ modelValue: true, template: null })

        expect((wrapper.vm as any).wrapVar('foo')).toBe('{{foo}}')
    })

    it('creates a new template with the built variables payload', async () => {
        const wrapper = mountEditor({ modelValue: true, template: null })
        ;(wrapper.vm as any).form.name = 'New'
        ;(wrapper.vm as any).form.slug = 'new'
        ;(wrapper.vm as any).form.subject_template = 'Hi {{name}}'
        await wrapper.vm.$nextTick()

        await (wrapper.vm as any).handleSave()

        expect(mockCreateEmailTemplate).toHaveBeenCalledWith(
            expect.objectContaining({
                name: 'New',
                slug: 'new',
                variables: [{ key: 'name', description: '' }],
            })
        )
        expect(showSuccess).toHaveBeenCalled()
        expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
        expect(wrapper.emitted('saved')).toBeTruthy()
    })

    it('updates an existing template, preserving typed descriptions', async () => {
        const template = makeTemplate()
        const wrapper = mountEditor({ modelValue: true, template })
        ;(wrapper.vm as any).variableDescriptions.name = 'Updated description'
        await wrapper.vm.$nextTick()

        await (wrapper.vm as any).handleSave()

        expect(mockUpdateEmailTemplate).toHaveBeenCalledWith(
            'tmpl-1',
            expect.objectContaining({
                variables: [{ key: 'name', description: 'Updated description' }],
            })
        )
        expect(showSuccess).toHaveBeenCalled()
    })

    it('routes save errors through the error handler and keeps the dialog open', async () => {
        mockCreateEmailTemplate.mockRejectedValue(new Error('save failed'))
        const wrapper = mountEditor({ modelValue: true, template: null })

        await (wrapper.vm as any).handleSave()

        expect(mockHandleError).toHaveBeenCalled()
        expect((wrapper.vm as any).saving).toBe(false)
        expect(wrapper.emitted('update:modelValue')).toBeFalsy()
    })

    it('shows a readonly notice for system templates', () => {
        mountEditor({ modelValue: true, template: makeTemplate({ template_type: 'system' }) })

        expect(body().text()).toContain('system_template_notice')
    })

    it('emits update:modelValue(false) when the cancel button is clicked', async () => {
        const wrapper = mountEditor({ modelValue: true, template: null })

        const cancelBtn = body()
            .findAll('button')
            .find(btn => btn.text().toLowerCase().includes('cancel'))
        await cancelBtn?.trigger('click')

        expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
    })
})
