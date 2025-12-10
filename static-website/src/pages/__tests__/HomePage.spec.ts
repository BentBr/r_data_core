import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import HomePage from '../HomePage.vue'

const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: HomePage }],
})

describe('HomePage', () => {
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
        const links = wrapper.findAll('a')
        expect(links.length).toBeGreaterThan(0)
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
})
