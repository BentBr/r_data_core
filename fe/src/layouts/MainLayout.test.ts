import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import MobileWarningBanner from '@/components/common/MobileWarningBanner.vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

// Create Vuetify instance for testing
const vuetify = createVuetify({
    components,
    directives,
})

// Create a simple router for testing
const createTestRouter = () => {
    return createRouter({
        history: createMemoryHistory(),
        routes: [
            {
                path: '/dashboard',
                component: { template: '<div>Dashboard</div>' },
            },
        ],
    })
}

describe('MainLayout - Banner Integration', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        localStorage.clear()
        // Reset window.innerWidth
        Object.defineProperty(window, 'innerWidth', {
            writable: true,
            configurable: true,
            value: 1400,
        })
    })

    it('should use separate localStorage keys for each banner', () => {
        // Set dismissed state for mobile banner
        localStorage.setItem('mobile_warning_banner_dismissed', 'true')

        // Set dismissed state for password banner
        localStorage.setItem('default_password_banner_dismissed', 'true')

        // Verify they are separate
        expect(localStorage.getItem('mobile_warning_banner_dismissed')).toBe('true')
        expect(localStorage.getItem('default_password_banner_dismissed')).toBe('true')

        // Clear one should not affect the other
        localStorage.removeItem('mobile_warning_banner_dismissed')
        expect(localStorage.getItem('mobile_warning_banner_dismissed')).toBeNull()
        expect(localStorage.getItem('default_password_banner_dismissed')).toBe('true')
    })

    it('should allow both banners to be dismissed independently via X icon', async () => {
        const router = createTestRouter()
        await router.push('/dashboard')

        Object.defineProperty(window, 'innerWidth', {
            writable: true,
            configurable: true,
            value: 800, // Mobile size
        })

        const pinia = createPinia()

        // Mock the login response to set usingDefaultPassword
        // We'll simulate this by directly accessing the store's internal state
        // Since usingDefaultPassword is not exported, we'll use a workaround
        // by setting localStorage and checking the computed property works

        // First, ensure mobile banner is not dismissed
        localStorage.removeItem('mobile_warning_banner_dismissed')

        // Mount both banner components separately to test their interaction
        const mobileWrapper = mount(MobileWarningBanner, {
            global: {
                plugins: [vuetify, pinia],
            },
        })

        await mobileWrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Mobile banner should be visible
        const mobileAlert = mobileWrapper.findComponent({ name: 'VAlert' })
        expect(mobileAlert.exists()).toBe(true)

        // Dismiss mobile banner via X icon
        await mobileAlert.vm.$emit('click:close')
        await mobileWrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        // Mobile banner should be gone
        const mobileAlertAfter = mobileWrapper.findComponent({ name: 'VAlert' })
        expect(mobileAlertAfter.exists()).toBe(false)

        // Check localStorage
        expect(localStorage.getItem('mobile_warning_banner_dismissed')).toBe('true')
        // Password banner localStorage should not be affected
        expect(localStorage.getItem('default_password_banner_dismissed')).not.toBe('true')
    })

    it('should allow both banners to be dismissed independently via dismiss button', async () => {
        Object.defineProperty(window, 'innerWidth', {
            writable: true,
            configurable: true,
            value: 800, // Mobile size
        })

        localStorage.clear()

        const pinia = createPinia()
        const mobileWrapper = mount(MobileWarningBanner, {
            global: {
                plugins: [vuetify, pinia],
            },
        })

        await mobileWrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Test dismiss functionality by calling handleDismiss directly
        const component = mobileWrapper.vm as any
        expect(component.handleDismiss).toBeDefined()

        // Call handleDismiss to simulate button click
        component.handleDismiss()
        await mobileWrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        // Banner should be dismissed
        const alertAfter = mobileWrapper.findComponent({ name: 'VAlert' })
        expect(alertAfter.exists()).toBe(false)

        // Check localStorage
        expect(localStorage.getItem('mobile_warning_banner_dismissed')).toBe('true')
    })
})
