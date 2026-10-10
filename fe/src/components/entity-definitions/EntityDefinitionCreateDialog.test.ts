/**
 * Mount strategy: Vuetify VDialog uses a teleport to document.body.
 * The global test-setup stubs <teleport> which hides dialog content.
 * We override that stub per-test (stubs: { teleport: false }) and attach
 * the wrapper to a real DOM node so the teleported content is queryable
 * via document.body. See SystemLogsViewer.test.ts for the same pattern.
 */
import { mount, VueWrapper } from '@vue/test-utils'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import EntityDefinitionCreateDialog from './EntityDefinitionCreateDialog.vue'

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (key: string) => key }),
}))

vi.mock('@/design-system/components', () => ({
    getDialogMaxWidth: () => '600px',
}))

vi.mock('@/components/common/IconPicker.vue', () => ({
    default: {
        name: 'IconPicker',
        props: ['modelValue', 'label'],
        emits: ['update:modelValue'],
        template: '<div class="icon-picker-stub"></div>',
    },
}))

const mountAttached = (props: Record<string, unknown> = {}) => {
    const container = document.createElement('div')
    document.body.appendChild(container)

    const wrapper = mount(EntityDefinitionCreateDialog, {
        attachTo: container,
        global: { stubs: { teleport: false } },
        props: {
            modelValue: true,
            loading: false,
            ...props,
        },
    })

    return { wrapper, container }
}

describe('EntityDefinitionCreateDialog', () => {
    let wrapper: VueWrapper
    let container: HTMLElement

    beforeEach(() => {
        vi.clearAllMocks()
    })

    afterEach(() => {
        wrapper.unmount()
        if (document.body.contains(container)) {
            document.body.removeChild(container)
        }
    })

    const mountComponent = (props: Record<string, unknown> = {}) => {
        const result = mountAttached(props)
        wrapper = result.wrapper
        container = result.container
        return wrapper
    }

    it('renders the create title', async () => {
        mountComponent()
        await wrapper.vm.$nextTick()

        expect(document.body.textContent).toContain('entity_definitions.create.title')
    })

    it('emits update:modelValue(false) when closeDialog is invoked', async () => {
        mountComponent()
        const vm = wrapper.vm as unknown as { closeDialog: () => void }
        vm.closeDialog()
        await wrapper.vm.$nextTick()

        expect(wrapper.emitted('update:modelValue')).toBeTruthy()
        expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
    })

    it('resets the form fields when closed', async () => {
        mountComponent()
        const vm = wrapper.vm as unknown as {
            form: { entity_type: string; display_name: string }
            closeDialog: () => void
        }
        vm.form.entity_type = 'Customer'
        vm.form.display_name = 'Customer'

        vm.closeDialog()
        await wrapper.vm.$nextTick()

        expect(vm.form.entity_type).toBe('')
        expect(vm.form.display_name).toBe('')
    })

    it('does not emit create when the form is invalid', async () => {
        mountComponent()
        const vm = wrapper.vm as unknown as {
            createEntityDefinition: () => void
            formValid: boolean
        }
        vm.formValid = false
        vm.createEntityDefinition()
        await wrapper.vm.$nextTick()

        expect(wrapper.emitted('create')).toBeFalsy()
    })

    it('emits create with form data and resets when the form is valid', async () => {
        mountComponent()
        const vm = wrapper.vm as unknown as {
            form: {
                entity_type: string
                display_name: string
                description: string
                group_name: string
                allow_children: boolean
                icon: string
                fields: unknown[]
                published: boolean
            }
            formValid: boolean
            createEntityDefinition: () => void
        }

        vm.form.entity_type = 'Customer'
        vm.form.display_name = 'Customer'
        vm.form.group_name = 'Sales'
        vm.form.allow_children = true
        vm.form.published = true
        vm.formValid = true

        vm.createEntityDefinition()
        await wrapper.vm.$nextTick()

        expect(wrapper.emitted('create')).toBeTruthy()
        const payload = wrapper.emitted('create')?.[0]?.[0] as {
            entity_type: string
            display_name: string
            group_name: string
            allow_children: boolean
            published: boolean
        }
        expect(payload.entity_type).toBe('Customer')
        expect(payload.display_name).toBe('Customer')
        expect(payload.group_name).toBe('Sales')
        expect(payload.allow_children).toBe(true)
        expect(payload.published).toBe(true)

        // closeDialog() is called after emitting, resetting the form
        expect(vm.form.entity_type).toBe('')
        expect(wrapper.emitted('update:modelValue')).toBeTruthy()
    })

    it('disables the create button while formValid is false', async () => {
        mountComponent()
        await wrapper.vm.$nextTick()

        const buttons = Array.from(document.body.querySelectorAll('button'))
        const createButton = buttons.find(b =>
            b.textContent?.includes('entity_definitions.create.create_button')
        )
        expect(createButton?.hasAttribute('disabled')).toBe(true)
    })

    it('shows a loading state on the create button when loading prop is true', async () => {
        mountComponent({ loading: true })
        await wrapper.vm.$nextTick()

        const buttons = Array.from(document.body.querySelectorAll('button'))
        const createButton = buttons.find(b =>
            b.textContent?.includes('entity_definitions.create.create_button')
        )
        expect(createButton?.className).toContain('v-btn--loading')
    })

    it('invokes closeDialog when cancel button is clicked', async () => {
        mountComponent()
        await wrapper.vm.$nextTick()

        const buttons = Array.from(document.body.querySelectorAll('button'))
        const cancelButton = buttons.find(b => b.textContent?.includes('common.cancel'))
        cancelButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
        await wrapper.vm.$nextTick()

        expect(wrapper.emitted('update:modelValue')).toBeTruthy()
        expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
    })
})
