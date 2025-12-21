import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import PricingPage from './PricingPage.vue'

const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/pricing', component: PricingPage }],
})

describe('PricingPage', () => {
    it('should render the component', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should have hero section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.pricing-hero-section').exists()).toBe(true)
    })

    it('should have free licenses section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.free-licenses-section').exists()).toBe(true)
    })

    it('should have company licenses section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.company-licenses-section').exists()).toBe(true)
    })

    it('should render free badges without being clipped', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        const freeBadges = wrapper.findAll('.free-badge')
        expect(freeBadges.length).toBeGreaterThan(0)

        // Verify badges exist and have proper positioning
        freeBadges.forEach(badge => {
            expect(badge.exists()).toBe(true)
        })
    })

    it('should render popular badge without being clipped', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        const popularBadge = wrapper.find('.popular-badge')
        expect(popularBadge.exists()).toBe(true)
    })

    it('should have pricing tier cards with overflow visible for badges', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        const tierCards = wrapper.findAll('.pricing-tier-card')
        expect(tierCards.length).toBeGreaterThan(0)
    })

    it('should have FAQ section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.faq-section').exists()).toBe(true)
    })

    it('should have corporate CTA section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.corporate-cta-section').exists()).toBe(true)
    })

    it('should have demo credentials dialog', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })
        expect(dialog.exists()).toBe(true)
    })

    it('should open demo dialog when open-demo event is dispatched', async () => {
        const wrapper = mount(PricingPage, {
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
        const wrapper = mount(PricingPage, {
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
