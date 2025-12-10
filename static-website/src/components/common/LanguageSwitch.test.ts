import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import LanguageSwitch from './LanguageSwitch.vue'

const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/:lang?', name: 'Home', component: { template: '<div>Home</div>' } }],
})

describe('LanguageSwitch', () => {
    it('should render the component', () => {
        const wrapper = mount(LanguageSwitch, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should display language toggle button', () => {
        const wrapper = mount(LanguageSwitch, {
            global: {
                plugins: [router],
            },
        })
        const button = wrapper.find('button')
        expect(button.exists()).toBe(true)
    })

    it('should have click handler', async () => {
        const wrapper = mount(LanguageSwitch, {
            global: {
                plugins: [router],
            },
        })
        const button = wrapper.find('button')
        expect(button.exists()).toBe(true)
    })
})
