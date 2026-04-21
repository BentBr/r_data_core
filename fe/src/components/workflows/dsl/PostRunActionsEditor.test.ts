import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import PostRunActionsEditor from './PostRunActionsEditor.vue'
import type { OnComplete } from '@/types/schemas/dsl'

const mockListEmailTemplates = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getDslFromOptions: vi.fn().mockResolvedValue({}),
        getDslToOptions: vi.fn().mockResolvedValue({}),
        getDslTransformOptions: vi.fn().mockResolvedValue({}),
        listEmailTemplates: (type?: string) => mockListEmailTemplates(type),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

vi.mock('@/stores/capabilities', () => ({
    useCapabilitiesStore: () => ({
        workflowMailConfigured: false,
    }),
}))

vi.mock('@/composables/useEntityDefinitions', () => ({
    useEntityDefinitions: () => ({
        entityDefinitions: { value: [] },
        loadEntityDefinitions: vi.fn().mockResolvedValue(undefined),
    }),
}))

/** Expand the first VExpansionPanel by clicking its title. */
async function expandPanel(wrapper: ReturnType<typeof mount>) {
    const title = wrapper.findComponent({ name: 'VExpansionPanelTitle' })
    await title.trigger('click')
    await nextTick()
    // Allow Vuetify's animation/rendering
    await new Promise(resolve => setTimeout(resolve, 50))
    await nextTick()
}

describe('PostRunActionsEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockListEmailTemplates.mockResolvedValue([])
    })

    it('renders the expansion panel', async () => {
        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: null },
        })

        await nextTick()

        const panel = wrapper.findComponent({ name: 'VExpansionPanel' })
        expect(panel.exists()).toBe(true)
    })

    it('renders empty state (no action rows) when no actions', async () => {
        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: null },
        })

        await expandPanel(wrapper)

        // No action rows rendered when no actions
        const actionRows = wrapper.findAll('.pa-3.border.rounded')
        expect(actionRows.length).toBe(0)
    })

    it('renders action rows when modelValue has actions', async () => {
        const onComplete: OnComplete = {
            actions: [
                {
                    type: 'send_email',
                    template_uuid: 'uuid-1',
                    to: [{ kind: 'const_string', value: 'admin@example.com' }],
                    cc: null,
                    condition: 'always',
                },
            ],
        }

        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: onComplete },
        })

        await expandPanel(wrapper)

        const actionRows = wrapper.findAll('.pa-3.border.rounded')
        expect(actionRows.length).toBe(1)
    })

    it('emits updated on_complete when add email action is clicked', async () => {
        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: null },
        })

        await expandPanel(wrapper)

        // Find the "add email action" button
        const buttons = wrapper.findAll('button')
        const addButton = buttons.find(b =>
            b.text().includes('workflows.dsl.post_run.add_email_action')
        )
        expect(addButton).toBeTruthy()

        await addButton!.trigger('click')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        expect(emitted).toBeTruthy()
        expect(emitted!.length).toBeGreaterThan(0)

        const emittedValue = emitted![0][0] as OnComplete
        expect(emittedValue).not.toBeNull()
        expect(emittedValue.actions.length).toBe(1)
        expect(emittedValue.actions[0].type).toBe('send_email')
    })

    it('shows condition dropdown for existing actions', async () => {
        const onComplete: OnComplete = {
            actions: [
                {
                    type: 'send_email',
                    template_uuid: 'uuid-1',
                    to: [{ kind: 'const_string', value: 'user@example.com' }],
                    cc: null,
                    condition: 'on_success',
                },
            ],
        }

        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: onComplete },
        })

        await expandPanel(wrapper)

        // The condition VSelect should be rendered inside the action row
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        expect(selects.length).toBeGreaterThan(0)

        // The condition select should have 3 options: always, on_success, on_failure
        const conditionSelect = selects[selects.length - 1]
        const items = conditionSelect.props('items') as Array<{ title: string; value: string }>
        const values = items.map(i => i.value)
        expect(values).toContain('always')
        expect(values).toContain('on_success')
        expect(values).toContain('on_failure')
    })

    it('emits updated on_complete when condition is changed', async () => {
        const onComplete: OnComplete = {
            actions: [
                {
                    type: 'send_email',
                    template_uuid: 'uuid-1',
                    to: [{ kind: 'const_string', value: 'admin@example.com' }],
                    cc: null,
                    condition: 'always',
                },
            ],
        }

        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: onComplete },
        })

        await expandPanel(wrapper)

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        expect(selects.length).toBeGreaterThan(0)

        // The condition select is the last one
        const conditionSelect = selects[selects.length - 1]
        await conditionSelect.vm.$emit('update:modelValue', 'on_failure')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        expect(emitted).toBeTruthy()

        const updated = emitted![0][0] as OnComplete
        expect(updated.actions[0].condition).toBe('on_failure')
    })

    it('emits null when last action is removed', async () => {
        const onComplete: OnComplete = {
            actions: [
                {
                    type: 'send_email',
                    template_uuid: 'uuid-1',
                    to: [{ kind: 'const_string', value: 'admin@example.com' }],
                    cc: null,
                    condition: 'always',
                },
            ],
        }

        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: onComplete },
        })

        await expandPanel(wrapper)

        // Find the remove action button inside the action row
        const actionRow = wrapper.find('.pa-3.border.rounded')
        expect(actionRow.exists()).toBe(true)

        const removeBtn = actionRow.find('button')
        expect(removeBtn.exists()).toBe(true)

        await removeBtn.trigger('click')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        expect(emitted).toBeTruthy()
        // When all actions are removed, emits null
        expect(emitted![0][0]).toBeNull()
    })

    it('calls listEmailTemplates(workflow) on mount', async () => {
        mount(PostRunActionsEditor, {
            props: { modelValue: null },
        })

        // Flush microtasks to allow async onMounted to run
        await nextTick()
        await nextTick()

        expect(mockListEmailTemplates).toHaveBeenCalledWith('workflow')
    })

    it('handles template loading failure gracefully', async () => {
        mockListEmailTemplates.mockRejectedValue(new Error('Network error'))

        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: null },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        // Component should render without error even if template loading fails
        expect(wrapper.exists()).toBe(true)
    })

    it('loaded email templates populate the template selector', async () => {
        mockListEmailTemplates.mockResolvedValue([
            { uuid: 'tmpl-1', name: 'Welcome Email', variables: [] },
            { uuid: 'tmpl-2', name: 'Farewell Email', variables: [] },
        ])

        const onComplete: OnComplete = {
            actions: [
                {
                    type: 'send_email',
                    template_uuid: '',
                    to: [],
                    cc: null,
                    condition: 'always',
                },
            ],
        }

        const wrapper = mount(PostRunActionsEditor, {
            props: { modelValue: onComplete },
        })

        // Allow async onMounted to resolve before expanding
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))
        await expandPanel(wrapper)

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        // First select is the template selector
        const templateSelect = selects[0]
        const items = templateSelect.props('items') as Array<{ title: string; value: string }>
        expect(items.length).toBe(2)
        expect(items[0].title).toBe('Welcome Email')
    })
})

/**
 * Note: The PostRunActionsEditor is conditionally shown by DslConfigurator
 * using `v-if="capabilitiesStore.workflowMailConfigured"`. When mail is not
 * configured the component is not mounted at all — that guard lives in
 * DslConfigurator.vue, not in PostRunActionsEditor itself.
 *
 * The `workflowMailConfigured: false` is already the default mock above, so
 * DslConfigurator should not render PostRunActionsEditor.
 */
describe('PostRunActionsEditor — workflowMailConfigured guard (DslConfigurator integration)', () => {
    it('is hidden in DslConfigurator when workflowMailConfigured is false', async () => {
        const { default: DslConfigurator } = await import('../DslConfigurator.vue')

        const wrapper = mount(DslConfigurator, {
            props: { modelValue: [] },
        })

        await nextTick()

        // PostRunActionsEditor should not be rendered when mail is not configured
        const postRunEditor = wrapper.findComponent(PostRunActionsEditor)
        expect(postRunEditor.exists()).toBe(false)
    })
})
