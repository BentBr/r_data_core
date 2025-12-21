import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import UseCasesPage from './UseCasesPage.vue'

const createTestRouter = () => {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/:lang(en|de)/use-cases', name: 'UseCases', component: UseCasesPage },
            { path: '/', redirect: '/en' },
        ],
    })
    // Initialize router to a valid route
    router.push('/en/use-cases')
    return router
}

describe('UseCasesPage', () => {
    let router: ReturnType<typeof createTestRouter>

    beforeEach(async () => {
        router = createTestRouter()
        await router.isReady()
    })
    it('should render the component', () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should have hero section', () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.hero-section').exists()).toBe(true)
    })

    it('should have intro section', () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.intro-section').exists()).toBe(true)
    })

    it('should have cases section', () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.cases-section').exists()).toBe(true)
    })

    it('should have CTA section', () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.cta-section').exists()).toBe(true)
    })

    it('should have demo credentials dialog', () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })
        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })
        expect(dialog.exists()).toBe(true)
    })

    it('should open demo dialog when open-demo event is dispatched', async () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })

        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })
        expect(dialog.exists()).toBe(true)

        // Initially dialog should be closed
        expect(dialog.props('modelValue')).toBe(false)

        // Dispatch the open-demo event
        window.dispatchEvent(new CustomEvent('open-demo'))
        await wrapper.vm.$nextTick()

        // Dialog should now be open
        expect(dialog.props('modelValue')).toBe(true)
    })

    it('should close demo dialog when update:modelValue is emitted', async () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })

        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })

        // Open the dialog first
        window.dispatchEvent(new CustomEvent('open-demo'))
        await wrapper.vm.$nextTick()
        expect(dialog.props('modelValue')).toBe(true)

        // Close the dialog
        dialog.vm.$emit('update:modelValue', false)
        await wrapper.vm.$nextTick()

        // Dialog should be closed
        expect(dialog.props('modelValue')).toBe(false)
    })

    it('should open demo dialog when CTA button is clicked', async () => {
        const wrapper = mount(UseCasesPage, {
            global: {
                plugins: [router],
            },
        })

        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })
        const buttons = wrapper.findAll('button')

        // Find the try demo button in CTA section
        const tryDemoButton = buttons.find(
            btn => btn.text().includes('Try demo') || btn.text().includes('Try Demo')
        )

        if (tryDemoButton) {
            await tryDemoButton.trigger('click')
            await wrapper.vm.$nextTick()

            // Dialog should be open
            expect(dialog.props('modelValue')).toBe(true)
        }
    })
})
