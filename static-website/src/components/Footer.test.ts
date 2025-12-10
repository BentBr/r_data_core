import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import Footer from './Footer.vue'

const router = createRouter({
    history: createMemoryHistory(),
    routes: [
        { path: '/', name: 'Home', component: { template: '<div>Home</div>' } },
        { path: '/about', name: 'About', component: { template: '<div>About</div>' } },
        { path: '/pricing', name: 'Pricing', component: { template: '<div>Pricing</div>' } },
    ],
})

describe('Footer', () => {
    it('should render the component', () => {
        const wrapper = mount(Footer, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should display site name', () => {
        const wrapper = mount(Footer, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.text()).toContain('RDataCore')
    })

    it('should display copyright text', () => {
        const wrapper = mount(Footer, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.text()).toContain('2025')
        expect(wrapper.text()).toContain('Slothlike')
    })

    it('should have navigation links', () => {
        const wrapper = mount(Footer, {
            global: {
                plugins: [router],
            },
        })
        const links = wrapper.findAll('a')
        expect(links.length).toBeGreaterThan(0)
    })

    it('should display "Built with Rust and Vue.js" text', () => {
        const wrapper = mount(Footer, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.text()).toContain('Rust')
        expect(wrapper.text()).toContain('Vue')
    })
})
