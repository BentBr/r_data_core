import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import Footer from './Footer.vue'

const createTestRouter = () => {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/:lang(en|de)', name: 'Home', component: { template: '<div>Home</div>' } },
            {
                path: '/:lang(en|de)/about',
                name: 'About',
                component: { template: '<div>About</div>' },
            },
            {
                path: '/:lang(en|de)/pricing',
                name: 'Pricing',
                component: { template: '<div>Pricing</div>' },
            },
            {
                path: '/:lang(en|de)/roadmap',
                name: 'Roadmap',
                component: { template: '<div>Roadmap</div>' },
            },
            {
                path: '/:lang(en|de)/use-cases',
                name: 'UseCases',
                component: { template: '<div>UseCases</div>' },
            },
            {
                path: '/:lang(en|de)/imprint',
                name: 'Imprint',
                component: { template: '<div>Imprint</div>' },
            },
            {
                path: '/:lang(en|de)/privacy',
                name: 'Privacy',
                component: { template: '<div>Privacy</div>' },
            },
            { path: '/', redirect: '/en' },
        ],
    })
    // Initialize router to a valid route
    router.push('/en')
    return router
}

describe('Footer', () => {
    let router: ReturnType<typeof createTestRouter>

    beforeEach(async () => {
        router = createTestRouter()
        await router.isReady()
    })
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
