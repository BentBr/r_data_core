import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import AboutPage from './AboutPage.vue'

const createTestRouter = () => {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/:lang(en|de)/about', name: 'About', component: AboutPage },
            { path: '/', redirect: '/en' },
        ],
    })
    // Initialize router to a valid route
    router.push('/en/about')
    return router
}

describe('AboutPage', () => {
    let router: ReturnType<typeof createTestRouter>

    beforeEach(async () => {
        router = createTestRouter()
        await router.isReady()
    })
    it('should render the component', () => {
        const wrapper = mount(AboutPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should have demo credentials dialog', () => {
        const wrapper = mount(AboutPage, {
            global: {
                plugins: [router],
            },
        })
        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })
        expect(dialog.exists()).toBe(true)
    })

    it('should open demo dialog when open-demo event is dispatched', async () => {
        const wrapper = mount(AboutPage, {
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
        const wrapper = mount(AboutPage, {
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
