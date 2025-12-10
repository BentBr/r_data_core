import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import Header from '../Header.vue'

const router = createRouter({
    history: createMemoryHistory(),
    routes: [
        { path: '/', name: 'Home', component: { template: '<div>Home</div>' } },
        { path: '/about', name: 'About', component: { template: '<div>About</div>' } },
        { path: '/pricing', name: 'Pricing', component: { template: '<div>Pricing</div>' } },
    ],
})

describe('Header', () => {
    it('should render the component', () => {
        const wrapper = mount(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should display site name', () => {
        const wrapper = mount(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.text()).toContain('RDataCore')
    })

    it('should display "by Slothlike" subline', () => {
        const wrapper = mount(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.text()).toContain('by Slothlike')
    })

    it('should have navigation links', () => {
        const wrapper = mount(Header, {
            global: {
                plugins: [router],
            },
        })
        const nav = wrapper.find('nav')
        expect(nav.exists()).toBe(true)
    })

    it('should have Try Demo button', () => {
        const wrapper = mount(Header, {
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

        const wrapper = mount(Header, {
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
        const wrapper = mount(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.findComponent({ name: 'ThemeToggle' }).exists()).toBe(true)
    })

    it('should have language switch', () => {
        const wrapper = mount(Header, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.findComponent({ name: 'LanguageSwitch' }).exists()).toBe(true)
    })
})
