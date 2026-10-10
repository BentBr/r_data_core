import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import SendEmailTransformEditor from './SendEmailTransformEditor.vue'
import type { Transform } from './contracts'
import type { EmailTemplate } from '@/api/clients/email-templates'

const mockListEmailTemplates: Mock = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        listEmailTemplates: (type?: string) => mockListEmailTemplates(type),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

function makeTransform(overrides: Partial<Extract<Transform, { type: 'send_email' }>> = {}) {
    return {
        type: 'send_email',
        template_uuid: '',
        to: [],
        target_status: '',
        ...overrides,
    } as Transform
}

function makeTemplate(overrides: Partial<EmailTemplate> = {}): EmailTemplate {
    return {
        uuid: 'tmpl-1',
        name: 'Welcome Email',
        slug: 'welcome-email',
        template_type: 'workflow',
        subject_template: 'Hi {{name}}',
        body_html_template: '<p>Hi {{name}}</p>',
        body_text_template: 'Hi {{name}}',
        variables: [{ key: 'name', description: 'Recipient name' }],
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        ...overrides,
    }
}

describe('SendEmailTransformEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockListEmailTemplates.mockResolvedValue([makeTemplate()])
    })

    it('renders info alert, template select and target status field', async () => {
        const wrapper = mount(SendEmailTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()

        const alerts = wrapper.findAllComponents({ name: 'VAlert' })
        expect(alerts.length).toBeGreaterThanOrEqual(1)

        const select = wrapper.findComponent({ name: 'VSelect' })
        expect(select.exists()).toBe(true)

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const targetStatusField = textFields.find(
            tf => (tf.props('label') as string) === 'workflows.dsl.send_email_target_status'
        )
        expect(targetStatusField).toBeTruthy()
    })

    it('loads email templates on mount and populates the template select', async () => {
        const wrapper = mount(SendEmailTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        expect(mockListEmailTemplates).toHaveBeenCalledWith('workflow')

        const select = wrapper.findComponent({ name: 'VSelect' })
        const items = select.props('items') as Array<{ title: string; value: string }>
        expect(items).toEqual([{ title: 'Welcome Email', value: 'tmpl-1' }])
    })

    it('clears the template list when loading templates fails', async () => {
        mockListEmailTemplates.mockRejectedValueOnce(new Error('network error'))
        const wrapper = mount(SendEmailTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const select = wrapper.findComponent({ name: 'VSelect' })
        expect(select.props('items')).toEqual([])
    })

    it('shows template variable hints when a template with variables is selected', async () => {
        const wrapper = mount(SendEmailTransformEditor, {
            props: { modelValue: makeTransform({ template_uuid: 'tmpl-1' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))
        await nextTick()

        expect(wrapper.text()).toContain('Recipient name')
        expect(wrapper.html()).toContain('{{name}}')
    })

    it('shows no variable hints when selected template has no variables array', async () => {
        mockListEmailTemplates.mockResolvedValue([makeTemplate({ variables: null })])
        const wrapper = mount(SendEmailTransformEditor, {
            props: { modelValue: makeTransform({ template_uuid: 'tmpl-1' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))
        await nextTick()

        const alerts = wrapper.findAllComponents({ name: 'VAlert' })
        expect(alerts.length).toBe(1) // only the top info alert
    })

    it('emits updated template_uuid and loads variables when a template is selected', async () => {
        const wrapper = mount(SendEmailTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const select = wrapper.findComponent({ name: 'VSelect' })
        await select.vm.$emit('update:modelValue', 'tmpl-1')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        expect(emitted).toBeTruthy()
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'send_email') {
            expect(updated.template_uuid).toBe('tmpl-1')
        }

        await wrapper.setProps({ modelValue: updated })
        await nextTick()
        expect(wrapper.text()).toContain('Recipient name')
    })

    it('updates target_status field', async () => {
        const wrapper = mount(SendEmailTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const targetStatusField = textFields.find(
            tf => (tf.props('label') as string) === 'workflows.dsl.send_email_target_status'
        )
        await targetStatusField!.vm.$emit('update:modelValue', 'sent')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'send_email') {
            expect(updated.target_status).toBe('sent')
        }
    })

    describe('To recipients', () => {
        it('adds a new recipient row with addTo', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: { modelValue: makeTransform() },
            })
            await nextTick()

            const addButton = wrapper
                .findAll('button')
                .find(b => b.text().includes('add_recipient'))
            expect(addButton).toBeTruthy()

            await addButton!.trigger('click')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.to).toEqual([{ kind: 'const_string', value: '' }])
            }
        })

        it('removes a recipient via removeTo', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({
                        to: [{ kind: 'const_string', value: 'a@test.com' }],
                    }),
                },
            })
            await nextTick()

            const deleteButton = wrapper.find('button[icon="mdi-delete"]')
            const deleteButtons = wrapper.findAllComponents({ name: 'VBtn' })
            const removeBtn = deleteButtons.find(b => b.props('icon') === 'mdi-delete')
            expect(removeBtn ?? deleteButton).toBeTruthy()
            await (removeBtn ?? deleteButton)!.trigger('click')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.to).toEqual([])
            }
        })

        it('switches a to-recipient kind between field and const_string', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({
                        to: [{ kind: 'const_string', value: 'a@test.com' }],
                    }),
                },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const kindSelect = selects.find(s => {
                const items = s.props('items') as string[] | undefined
                return (
                    Array.isArray(items) &&
                    items.includes('field') &&
                    items.includes('const_string')
                )
            })
            expect(kindSelect).toBeTruthy()

            await kindSelect!.vm.$emit('update:modelValue', 'field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.to).toEqual([{ kind: 'field', field: '' }])
            }
        })

        it('updates a to-recipient field value using a text field when no available fields', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({ to: [{ kind: 'field', field: '' }] }),
                },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            expect(valueField).toBeTruthy()
            await valueField!.vm.$emit('update:modelValue', 'email_field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.to).toEqual([{ kind: 'field', field: 'email_field' }])
            }
        })

        it('uses a select for to-recipient field when availableFields is provided', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({ to: [{ kind: 'field', field: '' }] }),
                    availableFields: ['email', 'secondary_email'],
                },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const fieldSelect = selects.find(s => {
                const items = s.props('items') as string[] | undefined
                return (
                    Array.isArray(items) &&
                    items.includes('email') &&
                    items.includes('secondary_email')
                )
            })
            expect(fieldSelect).toBeTruthy()

            await fieldSelect!.vm.$emit('update:modelValue', 'secondary_email')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.to).toEqual([{ kind: 'field', field: 'secondary_email' }])
            }
        })

        it('updates a to-recipient const value', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({ to: [{ kind: 'const_string', value: '' }] }),
                },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            await valueField!.vm.$emit('update:modelValue', 'static@test.com')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.to).toEqual([{ kind: 'const_string', value: 'static@test.com' }])
            }
        })
    })

    describe('CC recipients', () => {
        it('adds a new cc row with addCc', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: { modelValue: makeTransform() },
            })
            await nextTick()

            const addButton = wrapper
                .findAll('button')
                .find(b => b.text().includes('add_cc_recipient'))
            expect(addButton).toBeTruthy()

            await addButton!.trigger('click')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.cc).toEqual([{ kind: 'const_string', value: '' }])
            }
        })

        it('removes a cc entry and sets cc to undefined when the list becomes empty', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({
                        cc: [{ kind: 'const_string', value: 'cc@test.com' }],
                    }),
                },
            })
            await nextTick()

            const deleteButtons = wrapper.findAllComponents({ name: 'VBtn' })
            const ccDeleteButtons = deleteButtons.filter(b => b.props('icon') === 'mdi-delete')
            const removeBtn = ccDeleteButtons[ccDeleteButtons.length - 1]
            expect(removeBtn).toBeTruthy()
            await removeBtn!.trigger('click')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.cc).toBeUndefined()
            }
        })

        it('switches a cc kind between field and const_string', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({
                        cc: [{ kind: 'const_string', value: 'cc@test.com' }],
                    }),
                },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const kindSelects = selects.filter(s => {
                const items = s.props('items') as string[] | undefined
                return (
                    Array.isArray(items) &&
                    items.includes('field') &&
                    items.includes('const_string')
                )
            })
            const ccKindSelect = kindSelects[kindSelects.length - 1]
            expect(ccKindSelect).toBeTruthy()

            await ccKindSelect!.vm.$emit('update:modelValue', 'field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.cc).toEqual([{ kind: 'field', field: '' }])
            }
        })

        it('updates a cc field value', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({ cc: [{ kind: 'field', field: '' }] }),
                },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            await valueField!.vm.$emit('update:modelValue', 'cc_field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.cc).toEqual([{ kind: 'field', field: 'cc_field' }])
            }
        })

        it('updates a cc const value', async () => {
            const wrapper = mount(SendEmailTransformEditor, {
                props: {
                    modelValue: makeTransform({ cc: [{ kind: 'const_string', value: '' }] }),
                },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            await valueField!.vm.$emit('update:modelValue', 'cc-static@test.com')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'send_email') {
                expect(updated.cc).toEqual([{ kind: 'const_string', value: 'cc-static@test.com' }])
            }
        })
    })
})
