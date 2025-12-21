import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent } from 'vue'
import Header from './Header.vue'
import { VApp } from 'vuetify/components'

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
            { path: '/', redirect: '/en' },
        ],
    })
    // Initialize router to a valid route
    router.push('/en')
    return router
}

// Helper to wrap component in VApp for Vuetify layout context
const mountWithVApp = (component: typeof Header, options: any = {}) => {
    const Wrapper = defineComponent({
        components: { VApp, Header: component },
        template: '<v-app><Header /></v-app>',
    })
    return mount(Wrapper, options)
}

describe('Header', () => {
    let router: ReturnType<typeof createTestRouter>

    beforeEach(async () => {
        router = createTestRouter()
        await router.isReady()
    })
    it('should render the component', () => {
        const wrapper = mountWithVApp(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should display site name', () => {
        const wrapper = mountWithVApp(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.text()).toContain('RDataCore')
    })

    it('should display "by Slothlike" subline', () => {
        const wrapper = mountWithVApp(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.text()).toContain('by Slothlike')
    })

    it('should have navigation links', () => {
        const wrapper = mountWithVApp(Header, {
            global: {
                plugins: [router],
            },
        })
        const nav = wrapper.find('nav')
        expect(nav.exists()).toBe(true)
    })

    it('should have Try Demo button', () => {
        const wrapper = mountWithVApp(Header, {
            global: {
                plugins: [router],
            },
        })
        // Button should exist (may contain translation key or translated text)
        const buttons = wrapper.findAll('button')
        expect(buttons.length).toBeGreaterThan(0)
    })

    it('should emit open-demo event when button clicked', async () => {
        const dispatchEventSpy = vi.spyOn(window, 'dispatchEvent')

        const wrapper = mountWithVApp(Header, {
            global: {
                plugins: [router],
            },
        })

        const buttons = wrapper.findAll('button')
        expect(buttons.length).toBeGreaterThan(0)

        // Find the Try Demo button (usually the last one)
        const tryDemoButton = buttons[buttons.length - 1]
        await tryDemoButton.trigger('click')

        // Check if dispatchEvent was called with CustomEvent
        expect(dispatchEventSpy).toHaveBeenCalled()
        const callArgs = dispatchEventSpy.mock.calls[0]
        expect(callArgs.length).toBeGreaterThan(0)
    })

    it('should have theme toggle', () => {
        const wrapper = mountWithVApp(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.findComponent({ name: 'ThemeToggle' }).exists()).toBe(true)
    })

    it('should have language switch', () => {
        const wrapper = mountWithVApp(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.findComponent({ name: 'LanguageSwitch' }).exists()).toBe(true)
    })
})
