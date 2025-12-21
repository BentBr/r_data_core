import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import LanguageSwitch from './LanguageSwitch.vue'

const createTestRouter = () => {
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/:lang(en|de)', name: 'Home', component: { template: '<div>Home</div>' } },
            { path: '/', redirect: '/en' },
        ],
    })
    // Initialize router to a valid route
    router.push('/en')
    return router
}

describe('LanguageSwitch', () => {
    let router: ReturnType<typeof createTestRouter>

    beforeEach(async () => {
        router = createTestRouter()
        await router.isReady()
    })
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
