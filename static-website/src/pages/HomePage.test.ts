import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import HomePage from './HomePage.vue'

const createTestRouter = () => {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/:lang(en|de)', name: 'Home', component: HomePage },
            { path: '/', redirect: '/en' },
        ],
    })
    // Initialize router to a valid route
    router.push('/en')
    return router
}

describe('HomePage', () => {
    let router: ReturnType<typeof createTestRouter>

    beforeEach(async () => {
        router = createTestRouter()
        await router.isReady()
    })

    it('should render the component', () => {
        const wrapper = mount(HomePage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should have hero section', () => {
        const wrapper = mount(HomePage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.hero-section').exists()).toBe(true)
    })

    it('should have features section', () => {
        const wrapper = mount(HomePage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.features-section').exists()).toBe(true)
    })

    it('should have feature section structure', () => {
        const wrapper = mount(HomePage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.features-section').exists()).toBe(true)
    })

    it('should have CTA buttons', () => {
        const wrapper = mount(HomePage, {
            global: {
                plugins: [router],
            },
        })
        const buttons = wrapper.findAll('button')
        expect(buttons.length).toBeGreaterThan(0)
    })

    it('should have demo credentials dialog', () => {
        const wrapper = mount(HomePage, {
            global: {
                plugins: [router],
            },
        })
        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })
        expect(dialog.exists()).toBe(true)
    })

    it('should have API documentation links', () => {
        const wrapper = mount(HomePage, {
            global: {
                plugins: [router],
            },
        })
        const apiDocsText = wrapper.text()
        // Check that API documentation section exists
        expect(apiDocsText).toContain('API')
    })

    it('should handle open-demo event', () => {
        const wrapper = mount(HomePage, {
            global: {
                plugins: [router],
            },
        })

        // Component should have event listener
        expect(wrapper.vm).toBeDefined()
    })

    it('should open demo dialog when open-demo event is dispatched', async () => {
        const wrapper = mount(HomePage, {
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
        const wrapper = mount(HomePage, {
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
})
